//! Same-server remote fast rename/move via SFTP.

use super::super::super::events::TransferEvent;
use super::super::super::job::{FailedFile, FileTransferResult, TransferResults};
use super::super::super::worker::is_destination_parent_dir;
use super::super::BackendControl;
use crate::config::localization::t;
use crate::fs::ssh::SharedSshClient;
use anyhow::anyhow;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub async fn fast_remote_rename(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
    src_client: &SharedSshClient,
    dst_conn: &Option<SharedSshClient>,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    let is_dir_for_conn = |path: &Path, conn: &Option<SharedSshClient>| -> bool {
        if let Some(client) = conn {
            if let Ok(c) = client.0.lock()
                && let Ok(stat) = c.sftp.stat(path)
            {
                return stat.is_dir();
            }
            false
        } else {
            path.is_dir()
        }
    };

    let total_files = sources.len();
    let _ = control.event_tx.send(TransferEvent::ScanComplete {
        job_id: control.job_id,
        total_files,
        total_bytes: 0,
    });

    let mut results = TransferResults::default();
    for (idx, src) in sources.iter().enumerate() {
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }
        let name = src
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let dst = if is_destination_parent_dir(&sources, &destination_dir, |p| {
            is_dir_for_conn(p, dst_conn)
        }) {
            destination_dir.join(&name)
        } else {
            destination_dir.clone()
        };

        let start = Instant::now();
        let _ = control.event_tx.send(TransferEvent::FileStarted {
            job_id: control.job_id,
            file: src.clone(),
            index: idx,
        });

        if let Err(e) = src_client.rename_move(src, &dst) {
            let err_msg = t("error_remote_move_failed").replacen("{}", &e.to_string(), 1);
            let failed = FailedFile {
                src: src.clone(),
                dst: dst.clone(),
                error: err_msg.clone(),
                retries: 0,
            };
            results.failed_files.push(failed.clone());
            let _ = control.event_tx.send(TransferEvent::FileFailed {
                job_id: control.job_id,
                error: failed,
            });
            return Err(anyhow!(err_msg));
        }

        let result = FileTransferResult {
            src: src.clone(),
            dst,
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

    let _ = control.event_tx.send(TransferEvent::JobCompleted {
        job_id: control.job_id,
        results: results.clone(),
    });
    Ok(results)
}
