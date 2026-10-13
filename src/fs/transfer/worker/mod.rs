//! Transfer worker: orchestrates the phases of a local copy, move or
//! delete — fast renames (moves within one volume), scan, then the delete or
//! copy/move phase.
//!
//! Public API is stable at this module root:
//! - [`TransferWorker`]
//! - [`is_destination_parent_dir`]

mod delete_phase;
mod destination;
mod fast_move;
mod fs_helpers;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::Arc;

use super::control::JobControl;
use super::copy_phase::{CopyRequest, Endpoints, cleanup_source_dirs, run_copy_phase};
use super::events::TransferEvent;
use super::job::{TransferJob, TransferOperation, TransferResults};
use super::options::TransferOptions;
use super::scan::{ScanOutcome, ScanRequest, scan, scan_pairs, top_targets};
use crate::fs::vfs::LocalVfs;

pub(crate) use destination::ensure_dir;
pub use destination::is_destination_parent_dir;

pub struct TransferWorker {
    pub operation: TransferOperation,
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub options: TransferOptions,
    /// Explicit `(source, destination)` file pairs (see [`TransferJob::pairs`]).
    pub pairs: Option<Vec<(PathBuf, PathBuf)>>,
    /// Folders removed when empty after the job.
    pub prune_dirs: Vec<PathBuf>,
    /// Paths restored from the trash after the job.
    pub restore_after: Vec<PathBuf>,
    pub control: JobControl,
    pub active_conflict: Arc<crate::fs::transfer::conflict_slot::ConflictSlot>,
}

impl TransferWorker {
    /// Worker for `job`, reporting to `event_tx`.
    pub fn for_job(job: TransferJob, event_tx: crate::fs::transfer::events::EventSender) -> Self {
        let control = JobControl::for_job(&job, event_tx);
        Self {
            operation: job.operation,
            sources: job.sources,
            destination: job.destination,
            options: job.options,
            pairs: job.pairs,
            prune_dirs: job.prune_dirs,
            restore_after: job.restore_after,
            control,
            active_conflict: job.active_conflict,
        }
    }

    pub async fn run(self) -> Result<TransferResults, anyhow::Error> {
        let mut results = self.run_phases().await?;
        let mut prune = self.prune_dirs.clone();
        cleanup_source_dirs(&mut prune, &self.control, &LocalVfs);
        fs_helpers::restore_from_trash(&self.restore_after, &self.control, &mut results);
        self.control.job_completed(results)
    }

    /// The options of this run: LAN destinations get bigger buffers.
    fn effective_options(&self) -> TransferOptions {
        let mut options = self.options.clone();
        if super::network::is_lan_path(&self.destination) {
            options.buffer_size = crate::fs::transfer::options::BufferSize::_4MB;
        }
        options
    }

    /// Each source with the path it goes to.
    fn roots(&self) -> Vec<(PathBuf, PathBuf)> {
        match &self.pairs {
            Some(pairs) => pairs.clone(),
            None => top_targets(&self.sources, &self.destination, self.operation, &LocalVfs),
        }
    }

    fn scan(
        &self,
        roots: &[(PathBuf, PathBuf)],
        options: &TransferOptions,
    ) -> anyhow::Result<ScanOutcome> {
        if self.pairs.is_some() {
            return Ok(scan_pairs(roots, &self.control));
        }
        let request = ScanRequest {
            roots,
            operation: self.operation,
            options,
            src_fs: &LocalVfs,
            dst_fs: &LocalVfs,
            same_fs: true,
            dst_links: true,
        };
        scan(&request, &self.control)
    }

    async fn run_phases(&self) -> Result<TransferResults, anyhow::Error> {
        let ctl = &self.control;
        ctl.emit(TransferEvent::JobStarted { job_id: ctl.job_id });
        let options = self.effective_options();
        let mut results = TransferResults::default();

        let mut roots = self.roots();
        if self.operation == TransferOperation::Move {
            roots = fast_move::rename_roots(roots, &options, ctl, &mut results);
        }
        let scan = self.scan(&roots, &options)?;

        if self.operation == TransferOperation::Delete {
            return delete_phase::run_delete_phase(&self.sources, scan, &options, ctl).await;
        }
        destination::warn_low_space(&self.destination, scan.total_bytes, ctl);
        let request = CopyRequest {
            operation: self.operation,
            scan,
            options: &options,
            endpoints: Endpoints::local(),
            slot: Arc::clone(&self.active_conflict),
        };
        run_copy_phase(request, ctl, results).await
    }
}
