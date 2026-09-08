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
use super::BackendControl;
use anyhow::anyhow;
use std::path::PathBuf;

pub async fn run_ssh_job(
    operation: TransferOperation,
    sources: Vec<PathBuf>,
    destination: PathBuf,
    ssh: SshEndpoints,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    match operation {
        TransferOperation::Delete => run_ssh_delete(sources, ssh, control).await,
        TransferOperation::Copy => {
            run_ssh_copy_move(sources, destination, ssh, false, control).await
        }
        TransferOperation::Move => {
            run_ssh_copy_move(sources, destination, ssh, true, control).await
        }
        TransferOperation::Wipe
        | TransferOperation::Compress
        | TransferOperation::Extract
        | TransferOperation::ApplyCommand => {
            Err(anyhow!("{} is not available over SSH", operation.label()))
        }
    }
}
