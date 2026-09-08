//! SSH remote deletion operations.

use super::super::super::events::TransferEvent;
use super::super::super::job::{FailedFile, FileTransferResult, SshEndpoints, TransferResults};
use super::super::BackendControl;
use anyhow::anyhow;
use std::path::PathBuf;
use std::time::Instant;

pub async fn run_ssh_delete(
    sources: Vec<PathBuf>,
    ssh: SshEndpoints,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    let client = ssh
        .src
        .or(ssh.dst)
        .ok_or_else(|| anyhow!("SSH delete requires a connection"))?;

    let total = sources.len();
    let _ = control.event_tx.send(TransferEvent::ScanComplete {
        job_id: control.job_id,
        total_files: total,
        total_bytes: 0,
    });

    let mut results = TransferResults::default();

    for (idx, path) in sources.iter().enumerate() {
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }
        control.wait_if_paused();
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }

        let start = Instant::now();
        let _ = control.event_tx.send(TransferEvent::FileStarted {
            job_id: control.job_id,
            file: path.clone(),
            index: idx,
        });

        match client.delete_recursive(path) {
            Ok(()) => {
                let result = FileTransferResult {
                    src: path.clone(),
                    dst: PathBuf::new(),
                    size: 0,
                    src_hash: None,
                    dst_hash: None,
                    verified: true,
                    duration: start.elapsed(),
                };
                results.completed_files.push(result.clone());
                let _ = control.event_tx.send(TransferEvent::FileCompleted {
                    job_id: control.job_id,
                    result,
                });
            }
            Err(e) => {
                let failed = FailedFile {
                    src: path.clone(),
                    dst: PathBuf::new(),
                    error: e.to_string(),
                    retries: 0,
                };
                results.failed_files.push(failed.clone());
                let _ = control.event_tx.send(TransferEvent::FileFailed {
                    job_id: control.job_id,
                    error: failed,
                });
                return Err(anyhow!(e.to_string()));
            }
        }
    }

    let _ = control.event_tx.send(TransferEvent::JobCompleted {
        job_id: control.job_id,
        results: results.clone(),
    });
    Ok(results)
}
