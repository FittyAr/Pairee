use anyhow::anyhow;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::super::events::TransferEvent;
use super::super::job::{FailedFile, FileTransferResult, SkippedFile, TransferResults};
use super::super::options::TransferOptions;
use super::fs_helpers::{make_writable_helper, send_to_recycle_bin_helper};
use super::speed::spawn_speed_reporter;

/// Run the delete phase (recycle bin or permanent delete) and return results.
pub(super) async fn run_delete_phase(
    sources: &[PathBuf],
    scan_mappings: Vec<(PathBuf, PathBuf, u64)>,
    mut dirs_to_delete: Vec<PathBuf>,
    total_bytes: u64,
    options: &TransferOptions,
    job_id: Uuid,
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
    skip_file_flag: Arc<AtomicBool>,
    event_tx: mpsc::UnboundedSender<TransferEvent>,
) -> Result<TransferResults, anyhow::Error> {
    let mut results = TransferResults::default();
    let bytes_transferred_acc = Arc::new(AtomicU64::new(0));

    let _speed_reporter = spawn_speed_reporter(
        event_tx.clone(),
        job_id,
        Arc::clone(&bytes_transferred_acc),
        Arc::clone(&is_cancelled),
        total_bytes,
    );

    if options.delete_to_recycle_bin {
        for (idx, src) in sources.iter().enumerate() {
            if is_cancelled.load(Ordering::Relaxed) {
                return Err(anyhow!("Job cancelled"));
            }

            let delete_start = Instant::now();
            let _ = event_tx.send(TransferEvent::FileStarted {
                job_id,
                file: src.clone(),
                index: idx,
            });

            if let (Some(parent), Some(filename)) = (src.parent(), src.file_name()) {
                if let Some(filename_str) = filename.to_str() {
                    let _ = crate::fs::descriptions::remove_description(parent, filename_str);
                }
            }

            let res = send_to_recycle_bin_helper(src);

            if let Err(e) = res {
                let err_msg = e.to_string();
                results.failed_files.push(FailedFile {
                    src: src.clone(),
                    dst: PathBuf::new(),
                    error: err_msg.clone(),
                    retries: 0,
                });
                let _ = event_tx.send(TransferEvent::FileFailed {
                    job_id,
                    error: FailedFile {
                        src: src.clone(),
                        dst: PathBuf::new(),
                        error: err_msg,
                        retries: 0,
                    },
                });
                if options.halt_on_error {
                    return Err(anyhow!("Halt on error: Recycle Bin deletion failed"));
                }
            } else {
                let size = src.metadata().map(|m| m.len()).unwrap_or(0);
                let file_result = FileTransferResult {
                    src: src.clone(),
                    dst: PathBuf::new(),
                    size,
                    src_hash: None,
                    dst_hash: None,
                    verified: true,
                    duration: delete_start.elapsed(),
                };
                results.completed_files.push(file_result.clone());
                let _ = event_tx.send(TransferEvent::FileCompleted {
                    job_id,
                    result: file_result,
                });
                bytes_transferred_acc.fetch_add(size, Ordering::SeqCst);
            }
        }
    } else {
        for (idx, (src, _, size)) in scan_mappings.into_iter().enumerate() {
            if is_cancelled.load(Ordering::Relaxed) {
                return Err(anyhow!("Job cancelled"));
            }

            while is_paused.load(Ordering::Relaxed) {
                if is_cancelled.load(Ordering::Relaxed) {
                    return Err(anyhow!("Job cancelled"));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            if skip_file_flag.swap(false, Ordering::Relaxed) {
                results.skipped_files.push(SkippedFile {
                    src: src.clone(),
                    reason: "Skipped by user".to_string(),
                });
                let _ = event_tx.send(TransferEvent::FileSkipped {
                    job_id,
                    file: src.clone(),
                    reason: "Skipped by user".to_string(),
                });
                continue;
            }

            let delete_start = Instant::now();
            let _ = event_tx.send(TransferEvent::FileStarted {
                job_id,
                file: src.clone(),
                index: idx,
            });

            if let (Some(parent), Some(filename)) = (src.parent(), src.file_name()) {
                if let Some(filename_str) = filename.to_str() {
                    let _ = crate::fs::descriptions::remove_description(parent, filename_str);
                }
            }

            let mut res = std::fs::remove_file(&src);
            if res.is_err() {
                let _ = make_writable_helper(&src);
                res = std::fs::remove_file(&src);
            }

            if let Err(e) = res {
                let err_msg = e.to_string();
                results.failed_files.push(FailedFile {
                    src: src.clone(),
                    dst: PathBuf::new(),
                    error: err_msg.clone(),
                    retries: 0,
                });
                let _ = event_tx.send(TransferEvent::FileFailed {
                    job_id,
                    error: FailedFile {
                        src: src.clone(),
                        dst: PathBuf::new(),
                        error: err_msg,
                        retries: 0,
                    },
                });
                if options.halt_on_error {
                    return Err(anyhow!("Halt on error: Deletion failed"));
                }
            } else {
                let file_result = FileTransferResult {
                    src: src.clone(),
                    dst: PathBuf::new(),
                    size,
                    src_hash: None,
                    dst_hash: None,
                    verified: true,
                    duration: delete_start.elapsed(),
                };
                results.completed_files.push(file_result.clone());
                let _ = event_tx.send(TransferEvent::FileCompleted {
                    job_id,
                    result: file_result,
                });
                bytes_transferred_acc.fetch_add(size, Ordering::SeqCst);
            }
        }

        dirs_to_delete.sort_by_key(|p| std::cmp::Reverse(p.as_os_str().len()));
        for dir in dirs_to_delete {
            if let (Some(parent), Some(filename)) = (dir.parent(), dir.file_name()) {
                if let Some(filename_str) = filename.to_str() {
                    let _ = crate::fs::descriptions::remove_description(parent, filename_str);
                }
            }
            let mut res = std::fs::remove_dir(&dir);
            if res.is_err() {
                let _ = make_writable_helper(&dir);
                res = std::fs::remove_dir(&dir);
            }
            if let Err(e) = res {
                let err_msg = e.to_string();
                results.failed_files.push(FailedFile {
                    src: dir.clone(),
                    dst: PathBuf::new(),
                    error: err_msg.clone(),
                    retries: 0,
                });
                let _ = event_tx.send(TransferEvent::FileFailed {
                    job_id,
                    error: FailedFile {
                        src: dir.clone(),
                        dst: PathBuf::new(),
                        error: err_msg,
                        retries: 0,
                    },
                });
            }
        }
    }

    let _ = event_tx.send(TransferEvent::JobCompleted {
        job_id,
        results: results.clone(),
    });

    Ok(results)
}
