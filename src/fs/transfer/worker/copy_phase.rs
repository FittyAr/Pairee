use anyhow::anyhow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::super::conflict::resolve_filename_conflict;
use super::super::events::TransferEvent;
use super::super::job::{
    FailedFile, FileTransferResult, SkippedFile, TransferOperation, TransferResults,
};
use super::super::metadata::preserve_metadata;
use super::super::options::TransferOptions;
use super::super::pipeline::copy_file_pipelined;
use super::fs_helpers::make_writable_helper;
use super::speed::spawn_speed_reporter;

/// Run the copy/move transfer loop (conflicts, retries, symlink recreate, verify, cleanup).
pub(super) async fn run_copy_phase(
    operation: TransferOperation,
    scan_mappings: Vec<(PathBuf, PathBuf, u64)>,
    mut dirs_to_delete: Vec<PathBuf>,
    total_bytes: u64,
    options: &TransferOptions,
    job_id: Uuid,
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
    skip_file_flag: Arc<AtomicBool>,
    event_tx: mpsc::UnboundedSender<TransferEvent>,
    active_conflict: Arc<
        std::sync::Mutex<Option<crate::fs::transfer::conflict::ConflictResolution>>,
    >,
) -> Result<TransferResults, anyhow::Error> {
    let mut auto_resolution = None;
    let mut results = TransferResults::default();
    let bytes_transferred_acc = Arc::new(AtomicU64::new(0));

    let _speed_reporter = spawn_speed_reporter(
        event_tx.clone(),
        job_id,
        Arc::clone(&bytes_transferred_acc),
        Arc::clone(&is_cancelled),
        total_bytes,
    );

    for (idx, (src, mut dst, size)) in scan_mappings.into_iter().enumerate() {
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

        if dst.exists() {
            match resolve_existing_destination(
                &src,
                &mut dst,
                options,
                job_id,
                &is_cancelled,
                &event_tx,
                &active_conflict,
                &mut auto_resolution,
                &mut results,
            )
            .await?
            {
                ConflictAction::Skip => continue,
                ConflictAction::Proceed => {}
            }
        }

        let _ = event_tx.send(TransferEvent::FileStarted {
            job_id,
            file: src.clone(),
            index: idx,
        });

        let file_start = Instant::now();
        let transfer = transfer_one_file(
            &src,
            &dst,
            options,
            job_id,
            Arc::clone(&is_paused),
            Arc::clone(&is_cancelled),
            Arc::clone(&bytes_transferred_acc),
            &event_tx,
        )
        .await?;

        if !transfer.success {
            results.failed_files.push(FailedFile {
                src: src.clone(),
                dst: dst.clone(),
                error: transfer.last_error.clone(),
                retries: transfer.retries,
            });
            let _ = event_tx.send(TransferEvent::FileFailed {
                job_id,
                error: FailedFile {
                    src: src.clone(),
                    dst: dst.clone(),
                    error: transfer.last_error.clone(),
                    retries: transfer.retries,
                },
            });
            if options.halt_on_error {
                let _ = event_tx.send(TransferEvent::JobFailed {
                    job_id,
                    error: format!(
                        "Halt on error triggered by file failure: {}",
                        transfer.last_error
                    ),
                });
                return Err(anyhow::anyhow!("Halt on error: {}", transfer.last_error));
            }
            continue;
        }

        let _ = preserve_metadata(&src, &dst, options);

        let verified = true;
        if options.verify_after_copy
            && !verify_hashes(
                &src,
                &dst,
                size,
                &transfer.src_hash,
                &transfer.dst_hash,
                options,
                job_id,
                &event_tx,
                &mut results,
            )?
        {
            continue;
        }

        if operation == TransferOperation::Move && verified {
            let _ = std::fs::remove_file(&src);
        }

        let file_result = FileTransferResult {
            src: src.clone(),
            dst: dst.clone(),
            size,
            src_hash: transfer.src_hash.clone(),
            dst_hash: transfer.dst_hash.clone(),
            verified,
            duration: file_start.elapsed(),
        };

        results.completed_files.push(file_result.clone());

        let _ = event_tx.send(TransferEvent::FileCompleted {
            job_id,
            result: file_result,
        });
    }

    if operation == TransferOperation::Move {
        cleanup_source_dirs(&mut dirs_to_delete, &is_cancelled);
    }

    let _ = event_tx.send(TransferEvent::JobCompleted {
        job_id,
        results: results.clone(),
    });

    Ok(results)
}

enum ConflictAction {
    Skip,
    Proceed,
}

async fn resolve_existing_destination(
    src: &Path,
    dst: &mut PathBuf,
    options: &TransferOptions,
    job_id: Uuid,
    is_cancelled: &AtomicBool,
    event_tx: &mpsc::UnboundedSender<TransferEvent>,
    active_conflict: &Arc<
        std::sync::Mutex<Option<crate::fs::transfer::conflict::ConflictResolution>>,
    >,
    auto_resolution: &mut Option<crate::fs::transfer::conflict::ConflictResolution>,
    results: &mut TransferResults,
) -> Result<ConflictAction, anyhow::Error> {
    let mut resolution = options.conflict_resolution.clone();
    if resolution == "ask" {
        let chosen = if let Some(auto_res) = *auto_resolution {
            auto_res
        } else {
            let _ = event_tx.send(TransferEvent::ConflictDetected {
                job_id,
                file: dst.clone(),
                conflict: crate::fs::transfer::conflict::ConflictInfo {
                    src_path: src.to_path_buf(),
                    dst_path: dst.clone(),
                    src_size: src.metadata().map(|m| m.len()).unwrap_or(0),
                    dst_size: dst.metadata().map(|m| m.len()).unwrap_or(0),
                    src_modified: src.metadata().and_then(|m| m.modified()).ok(),
                    dst_modified: dst.metadata().and_then(|m| m.modified()).ok(),
                },
            });

            {
                let mut guard = active_conflict.lock().unwrap();
                *guard = None;
            }

            while active_conflict.lock().unwrap().is_none() {
                if is_cancelled.load(Ordering::Relaxed) {
                    return Err(anyhow!("Job cancelled"));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            let ch = (*active_conflict.lock().unwrap())
                .unwrap_or(crate::fs::transfer::conflict::ConflictResolution::Skip);
            match ch {
                crate::fs::transfer::conflict::ConflictResolution::OverwriteAll
                | crate::fs::transfer::conflict::ConflictResolution::OverwriteOlderAll
                | crate::fs::transfer::conflict::ConflictResolution::SkipAll
                | crate::fs::transfer::conflict::ConflictResolution::RenameAll => {
                    *auto_resolution = Some(ch);
                }
                _ => {}
            }
            ch
        };

        resolution = match chosen {
            crate::fs::transfer::conflict::ConflictResolution::Overwrite
            | crate::fs::transfer::conflict::ConflictResolution::OverwriteAll => {
                "overwrite".to_string()
            }
            crate::fs::transfer::conflict::ConflictResolution::OverwriteOlder
            | crate::fs::transfer::conflict::ConflictResolution::OverwriteOlderAll => {
                "overwrite_older".to_string()
            }
            crate::fs::transfer::conflict::ConflictResolution::Rename
            | crate::fs::transfer::conflict::ConflictResolution::RenameAll
            | crate::fs::transfer::conflict::ConflictResolution::KeepBoth => "rename".to_string(),
            crate::fs::transfer::conflict::ConflictResolution::Cancel => {
                is_cancelled.store(true, Ordering::SeqCst);
                return Err(anyhow!("Job cancelled"));
            }
            _ => "skip".to_string(),
        };
    }

    match resolution.as_str() {
        "skip" => {
            results.skipped_files.push(SkippedFile {
                src: src.to_path_buf(),
                reason: "File already exists (skipped)".to_string(),
            });
            let _ = event_tx.send(TransferEvent::FileSkipped {
                job_id,
                file: src.to_path_buf(),
                reason: "File already exists".to_string(),
            });
            Ok(ConflictAction::Skip)
        }
        "rename" | "keep_both" => {
            *dst = resolve_filename_conflict(dst);
            Ok(ConflictAction::Proceed)
        }
        "overwrite_older" => {
            let src_time = src.metadata().and_then(|m| m.modified()).ok();
            let dst_time = dst.metadata().and_then(|m| m.modified()).ok();
            if let (Some(s_time), Some(d_time)) = (src_time, dst_time) {
                if s_time <= d_time {
                    results.skipped_files.push(SkippedFile {
                        src: src.to_path_buf(),
                        reason: "Destination is newer or equal (skipped)".to_string(),
                    });
                    let _ = event_tx.send(TransferEvent::FileSkipped {
                        job_id,
                        file: src.to_path_buf(),
                        reason: "Destination is newer or equal".to_string(),
                    });
                    return Ok(ConflictAction::Skip);
                }
            }
            Ok(ConflictAction::Proceed)
        }
        _ => Ok(ConflictAction::Proceed), // Overwrite
    }
}

struct TransferOutcome {
    success: bool,
    last_error: String,
    retries: u32,
    src_hash: Option<String>,
    dst_hash: Option<String>,
}

async fn transfer_one_file(
    src: &Path,
    dst: &Path,
    options: &TransferOptions,
    job_id: Uuid,
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
    bytes_transferred_acc: Arc<AtomicU64>,
    event_tx: &mpsc::UnboundedSender<TransferEvent>,
) -> Result<TransferOutcome, anyhow::Error> {
    let mut retries = 0u32;
    let mut copy_success = false;
    let mut last_error = String::new();
    let mut src_hash = None;
    let mut dst_hash = None;

    let is_symlink = src.is_symlink();
    let recreate_link = is_symlink && !options.follow_symlinks;

    if recreate_link {
        match recreate_symlink(src, dst) {
            Ok(()) => copy_success = true,
            Err(e) => last_error = format!("Error creating symlink: {}", e),
        }
    } else {
        while retries <= options.max_retries {
            if is_cancelled.load(Ordering::Relaxed) {
                return Err(anyhow!("Job cancelled"));
            }

            match copy_file_pipelined(
                src,
                dst,
                options,
                event_tx,
                job_id,
                Arc::clone(&is_paused),
                Arc::clone(&is_cancelled),
                Arc::clone(&bytes_transferred_acc),
            )
            .await
            {
                Ok((s_hash, d_hash)) => {
                    src_hash = s_hash;
                    dst_hash = d_hash;
                    copy_success = true;
                    break;
                }
                Err(e) => {
                    retries += 1;
                    last_error = e.to_string();
                    if retries <= options.max_retries {
                        // Exponential backoff 100ms… capped at 30s; clamp shift to avoid overflow.
                        let shift = retries.min(20);
                        let backoff_ms = 100u64.saturating_mul(1u64 << shift);
                        tokio::time::sleep(Duration::from_millis(backoff_ms.min(30_000))).await;
                    }
                }
            }
        }
    }

    Ok(TransferOutcome {
        success: copy_success,
        last_error,
        retries,
        src_hash,
        dst_hash,
    })
}

fn recreate_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    let target = std::fs::read_link(src)?;
    // Resolve relative targets against the *source* parent so the new symlink
    // points at the same file as the original (not a different path under dst).
    let resolved_target = if target.is_relative() {
        src.parent()
            .map(|p| p.join(&target))
            .unwrap_or_else(|| target.clone())
    } else {
        target.clone()
    };
    #[cfg(target_os = "windows")]
    {
        let is_dir = resolved_target.is_dir();
        if dst.exists() {
            let _ = std::fs::remove_file(dst);
            let _ = std::fs::remove_dir_all(dst);
        }
        if is_dir {
            std::os::windows::fs::symlink_dir(&resolved_target, dst)?;
        } else {
            std::os::windows::fs::symlink_file(&resolved_target, dst)?;
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if dst.exists() {
            let _ = std::fs::remove_file(dst);
        }
        std::os::unix::fs::symlink(&resolved_target, dst)?;
    }
    Ok(())
}

fn verify_hashes(
    src: &Path,
    dst: &Path,
    size: u64,
    src_hash: &Option<String>,
    dst_hash: &Option<String>,
    options: &TransferOptions,
    job_id: Uuid,
    event_tx: &mpsc::UnboundedSender<TransferEvent>,
    results: &mut TransferResults,
) -> Result<bool, anyhow::Error> {
    let _ = event_tx.send(TransferEvent::VerifyStarted {
        job_id,
        file: src.to_path_buf(),
        algorithm: options.hash_algorithm.as_str().to_string(),
    });

    if let (Some(sh), Some(dh)) = (src_hash.as_ref(), dst_hash.as_ref()) {
        let _ = event_tx.send(TransferEvent::VerifyProgress {
            job_id,
            bytes_verified: size,
            bytes_total: size,
        });

        if sh != dh {
            results.failed_files.push(FailedFile {
                src: src.to_path_buf(),
                dst: dst.to_path_buf(),
                error: "Hash verification mismatch".to_string(),
                retries: 0,
            });
            let _ = event_tx.send(TransferEvent::FileFailed {
                job_id,
                error: FailedFile {
                    src: src.to_path_buf(),
                    dst: dst.to_path_buf(),
                    error: "Hash verification mismatch".to_string(),
                    retries: 0,
                },
            });
            if options.halt_on_error {
                let _ = event_tx.send(TransferEvent::JobFailed {
                    job_id,
                    error: "Halt on error triggered by hash mismatch".to_string(),
                });
                return Err(anyhow::anyhow!("Halt on error: Hash mismatch"));
            }
            return Ok(false);
        }
    }
    Ok(true)
}

fn cleanup_source_dirs(dirs_to_delete: &mut [PathBuf], is_cancelled: &AtomicBool) {
    dirs_to_delete.sort_by_key(|p| std::cmp::Reverse(p.as_os_str().len()));
    for dir in dirs_to_delete.iter() {
        if is_cancelled.load(Ordering::Relaxed) {
            break;
        }
        if let (Some(parent), Some(filename)) = (dir.parent(), dir.file_name()) {
            if let Some(filename_str) = filename.to_str() {
                let _ = crate::fs::descriptions::remove_description(parent, filename_str);
            }
        }
        if std::fs::remove_dir(dir).is_err() {
            let _ = make_writable_helper(dir);
            let _ = std::fs::remove_dir(dir);
        }
    }
}
