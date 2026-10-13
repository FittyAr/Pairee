//! Copy phase of copies and moves — local, or between a local panel and an
//! SFTP server — as a pipeline of steps:
//!
//! 1. [`CopyJob::prepare`] — destination folders, entries the scan refused
//!    or skipped;
//! 2. per file ([`CopyJob::file`]): pause / user skip → same-file guard and
//!    conflict resolution → atomic transfer with retries (the local pipeline
//!    of [`single_file`], or a [`stream`] through the [`Vfs`] adapters) →
//!    metadata → removal of a moved source → result;
//! 3. after a move, removal of the emptied source folders.

mod cleanup;
mod file_step;
mod retry;
mod single_file;
mod stream;
#[cfg(test)]
mod tests;
mod verify_step;

use std::sync::Arc;

use super::control::JobControl;
use super::job::{TransferOperation, TransferResults};
use super::options::TransferOptions;
use super::scan::ScanOutcome;
use crate::fs::vfs::{LocalVfs, Vfs};

pub use cleanup::cleanup_source_dirs;
use file_step::CopyJob;

/// The two sides of a copy.
#[derive(Clone)]
pub struct Endpoints {
    pub src: Arc<dyn Vfs>,
    pub dst: Arc<dyn Vfs>,
    /// One filesystem: the same-file guard applies.
    pub same_fs: bool,
    /// Both sides are the local disk: files go through the local pipeline
    /// (Direct I/O, resume, links, sparse files, copy-on-write, trash).
    pub local: bool,
}

impl Endpoints {
    /// Both sides on the local disk.
    pub fn local() -> Self {
        Self {
            src: Arc::new(LocalVfs),
            dst: Arc::new(LocalVfs),
            same_fs: true,
            local: true,
        }
    }
}

/// What the copy phase works on.
pub struct CopyRequest<'a> {
    pub operation: TransferOperation,
    pub scan: ScanOutcome,
    pub options: &'a TransferOptions,
    pub endpoints: Endpoints,
    pub slot: Arc<crate::fs::transfer::conflict_slot::ConflictSlot>,
}

/// Runs the copy/move of everything the scan found; `results` holds what
/// the job already did (fast renames). The caller reports the end of the
/// job.
pub async fn run_copy_phase(
    request: CopyRequest<'_>,
    ctl: &JobControl,
    results: TransferResults,
) -> Result<TransferResults, anyhow::Error> {
    let ScanOutcome {
        mappings,
        dst_dirs,
        mut dirs_to_delete,
        refused,
        skipped,
        total_bytes,
    } = request.scan;
    let endpoints = request.endpoints;
    let mut job = CopyJob::new(
        request.operation,
        request.options,
        ctl,
        (&request.slot, &endpoints),
        results,
        total_bytes,
    );
    job.prepare(&dst_dirs, refused, skipped)?;
    for (index, mapping) in mappings.into_iter().enumerate() {
        job.file(index, mapping).await?;
    }
    let results = job.finish();
    if request.operation == TransferOperation::Move {
        cleanup_source_dirs(&mut dirs_to_delete, ctl, endpoints.src.as_ref());
    }
    Ok(results)
}
