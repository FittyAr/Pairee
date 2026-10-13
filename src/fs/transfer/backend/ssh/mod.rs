//! SSH/SFTP transfer backend facade.
//!
//! Copy and move run the shared scan and copy phase over the SFTP adapter
//! (blocking libssh2 calls happen on Tokio's blocking pool); a move within
//! one server is a series of renames; delete removes each source tree.

pub mod copy_move;
pub mod delete;
pub mod rename;

pub use copy_move::run_ssh_copy_move;
pub use delete::run_ssh_delete;
pub use rename::fast_remote_rename;

use super::super::job::{SshEndpoints, TransferJob, TransferOperation, TransferResults};
use super::super::options::TransferOptions;
use crate::fs::transfer::conflict_slot::ConflictSlot;
use crate::fs::transfer::control::JobControl;
use anyhow::anyhow;
use std::path::PathBuf;
use std::sync::Arc;

/// What an SSH job needs from its [`TransferJob`].
#[derive(Clone)]
pub struct SshJob {
    pub operation: TransferOperation,
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub ssh: SshEndpoints,
    pub options: TransferOptions,
    pub slot: Arc<ConflictSlot>,
}

impl SshJob {
    pub fn of(job: TransferJob, ssh: SshEndpoints) -> Self {
        Self {
            operation: job.operation,
            sources: job.sources,
            destination: job.destination,
            ssh,
            options: job.options,
            slot: job.active_conflict,
        }
    }
}

/// Runs an SSH job.
pub async fn run_ssh_job(
    job: SshJob,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    match job.operation {
        TransferOperation::Copy | TransferOperation::Move => run_ssh_copy_move(job, control).await,
        TransferOperation::Delete => {
            tokio::task::spawn_blocking(move || run_ssh_delete(job.sources, job.ssh, control))
                .await
                .map_err(|e| anyhow!("SSH transfer task failed: {e}"))?
        }
        TransferOperation::Wipe
        | TransferOperation::Compress
        | TransferOperation::Extract
        | TransferOperation::ApplyCommand
        | TransferOperation::Restore => Err(anyhow!(
            "{} is not available over SSH",
            job.operation.label()
        )),
    }
}
