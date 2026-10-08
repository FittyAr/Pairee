//! Transfer worker: orchestrates scan → delete or copy/move phases.
//!
//! Public API is stable at this module root:
//! - [`TransferWorker`]
//! - [`is_destination_parent_dir`]

mod copy_phase;
mod delete_phase;
mod destination;
mod fs_helpers;
mod scan;
mod speed;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::Arc;

use super::control::JobControl;
use super::events::TransferEvent;
use super::job::{TransferJob, TransferOperation, TransferResults};
use super::options::TransferOptions;

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
            control,
            active_conflict: job.active_conflict,
        }
    }

    pub async fn run(self) -> Result<TransferResults, anyhow::Error> {
        let results = self.run_phases().await?;
        let mut prune = self.prune_dirs.clone();
        copy_phase::cleanup_source_dirs(&mut prune, &self.control);
        self.control.job_completed(results)
    }

    async fn run_phases(&self) -> Result<TransferResults, anyhow::Error> {
        let ctl = &self.control;
        ctl.emit(TransferEvent::JobStarted { job_id: ctl.job_id });

        // LAN destinations get bigger buffers.
        let mut options = self.options.clone();
        if super::network::is_lan_path(&self.destination) {
            options.buffer_size = crate::fs::transfer::options::BufferSize::_4MB;
        }

        // Phase 1: scan.
        let scan = match &self.pairs {
            Some(pairs) => scan::scan_pairs(pairs, ctl),
            None => scan::scan(
                &self.sources,
                &self.destination,
                self.operation,
                &options,
                ctl,
            )?,
        };

        if self.operation == TransferOperation::Delete {
            return delete_phase::run_delete_phase(&self.sources, scan, &options, ctl).await;
        }

        // Warn (without stopping) when the destination looks too small.
        if let Ok(free_space) = super::network::get_free_space(&self.destination)
            && free_space < scan.total_bytes
        {
            ctl.emit(TransferEvent::FileSkipped {
                job_id: ctl.job_id,
                file: self.destination.clone(),
                reason: format!(
                    "Warning: Low disk space. Required: {}, Available: {}",
                    bytesize::ByteSize(scan.total_bytes),
                    bytesize::ByteSize(free_space)
                ),
            });
        }

        // Phase 2: copy / move.
        copy_phase::run_copy_phase(
            self.operation,
            scan,
            &options,
            ctl,
            Arc::clone(&self.active_conflict),
        )
        .await
    }
}
