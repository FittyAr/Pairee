//! SSH/SFTP transfer backend facade.
//!
//! Ports the former `ops_worker::copy_move` / delete paths onto
//! [`TransferEvent`] so the UI uses a single progress model.

pub mod copy_move;
pub mod delete;
pub mod rename;

pub use copy_move::run_ssh_copy_move;
pub use delete::run_ssh_delete;
pub use rename::fast_remote_rename;

use super::super::job::{SshEndpoints, TransferOperation, TransferResults};
use crate::fs::transfer::control::JobControl;
use anyhow::anyhow;
use std::path::PathBuf;

/// Runs an SSH job. libssh2 calls are blocking, so the whole job executes
/// on Tokio's blocking pool instead of stalling an async worker thread.
pub async fn run_ssh_job(
    operation: TransferOperation,
    sources: Vec<PathBuf>,
    destination: PathBuf,
    ssh: SshEndpoints,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    tokio::task::spawn_blocking(move || match operation {
        TransferOperation::Delete => run_ssh_delete(sources, ssh, control),
        TransferOperation::Copy => run_ssh_copy_move(sources, destination, ssh, false, control),
        TransferOperation::Move => run_ssh_copy_move(sources, destination, ssh, true, control),
        TransferOperation::Wipe
        | TransferOperation::Compress
        | TransferOperation::Extract
        | TransferOperation::ApplyCommand => {
            Err(anyhow!("{} is not available over SSH", operation.label()))
        }
    })
    .await
    .map_err(|e| anyhow!("SSH transfer task failed: {e}"))?
}
