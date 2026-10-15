//! Remote transfer backend facade (SFTP, FTP / FTPS, WebDAV, S3).
//!
//! Copy and move run the shared scan and copy phase over the connections'
//! adapters (blocking network calls happen on Tokio's blocking pool); a move within
//! one server is a series of renames; delete removes each source tree.

pub mod copy_move;
pub mod delete;
pub mod rename;

pub use copy_move::run_remote_copy_move;
pub use delete::run_remote_delete;
pub use rename::fast_remote_rename;

use super::super::job::{
    DestinationKind, RemoteEndpoints, TransferJob, TransferOperation, TransferResults,
};
use super::super::options::TransferOptions;
use crate::fs::transfer::conflict_slot::ConflictSlot;
use crate::fs::transfer::control::JobControl;
use anyhow::anyhow;
use std::path::PathBuf;
use std::sync::Arc;

/// What a remote job needs from its [`TransferJob`].
#[derive(Clone)]
pub struct RemoteJob {
    pub operation: TransferOperation,
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub destination_kind: DestinationKind,
    pub remote: RemoteEndpoints,
    pub options: TransferOptions,
    pub slot: Arc<ConflictSlot>,
}

impl RemoteJob {
    pub fn of(job: TransferJob, remote: RemoteEndpoints) -> Self {
        Self {
            operation: job.operation,
            sources: job.sources,
            destination: job.destination,
            destination_kind: job.destination_kind,
            remote,
            options: job.options.for_operation(job.operation),
            slot: job.active_conflict,
        }
    }
}

/// Runs a remote job.
pub async fn run_remote_job(
    job: RemoteJob,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    match job.operation {
        TransferOperation::Copy | TransferOperation::Move => {
            run_remote_copy_move(job, control).await
        }
        TransferOperation::Delete => {
            tokio::task::spawn_blocking(move || run_remote_delete(job.sources, job.remote, control))
                .await
                .map_err(|e| anyhow!("Remote transfer task failed: {e}"))?
        }
        TransferOperation::Wipe
        | TransferOperation::Compress
        | TransferOperation::Extract
        | TransferOperation::ApplyCommand
        | TransferOperation::Restore => Err(anyhow!(
            "{} is not available on remote connections",
            job.operation.label()
        )),
    }
}
