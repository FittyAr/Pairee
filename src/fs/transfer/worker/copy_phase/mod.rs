//! Copy phase orchestrator for local transfer operations.

pub mod conflict;
pub mod single_file;
pub mod verify_cleanup;

use anyhow::anyhow;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::super::events::TransferEvent;
use super::super::job::{
    FailedFile, FileTransferResult, SkippedFile, TransferOperation, TransferResults,
};
use super::super::metadata::preserve_metadata;
use super::super::options::TransferOptions;
use super::speed::spawn_speed_reporter;

pub use conflict::{ConflictAction, resolve_existing_destination};
pub use single_file::transfer_one_file;
pub use verify_cleanup::{cleanup_source_dirs, verify_hashes};

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
                return Err(anyhow!("Halt on error: {}", transfer.last_error));
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
