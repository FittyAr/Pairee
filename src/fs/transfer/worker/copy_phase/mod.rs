//! Copy phase orchestrator for local transfer operations.

pub mod conflict;
pub mod single_file;
pub mod verify_cleanup;

use anyhow::anyhow;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Instant;

use super::super::control::JobControl;
use super::super::events::TransferEvent;
use super::super::job::{FailedFile, FileTransferResult, TransferOperation, TransferResults};
use super::super::metadata::preserve_metadata;
use super::super::options::TransferOptions;
use super::scan::ScanOutcome;
use super::speed::spawn_speed_reporter;

pub use conflict::{ConflictAction, resolve_existing_destination};
pub use single_file::transfer_one_file;
pub use verify_cleanup::{cleanup_source_dirs, verify_hashes};

/// Run the copy/move transfer loop (conflicts, retries, symlink recreate,
/// verify, cleanup). The caller reports the end of the job.
pub(super) async fn run_copy_phase(
    operation: TransferOperation,
    scan: ScanOutcome,
    options: &TransferOptions,
    ctl: &JobControl,
    active_conflict: Arc<crate::fs::transfer::conflict_slot::ConflictSlot>,
) -> Result<TransferResults, anyhow::Error> {
    let mut auto_resolution = None;
    let mut results = TransferResults {
        created_dirs: scan.created_dirs,
        ..TransferResults::default()
    };
    let copied_bytes = Arc::new(AtomicU64::new(0));
    let _speed_reporter = spawn_speed_reporter(ctl, Arc::clone(&copied_bytes), scan.total_bytes);

    for (idx, (src, mut dst, size)) in scan.mappings.into_iter().enumerate() {
        ctl.wait_if_paused().await?;
        if ctl.take_user_skip(&src, &mut results) {
            continue;
        }
        if dst.exists() {
            let action = resolve_existing_destination(
                &src,
                &mut dst,
                options,
                ctl,
                &active_conflict,
                &mut auto_resolution,
                &mut results,
            )
            .await?;
            if let ConflictAction::Skip = action {
                continue;
            }
        }
        let replaced = dst.symlink_metadata().is_ok();

        ctl.file_started(&src, idx);
        let file_start = Instant::now();
        let transfer =
            transfer_one_file(&src, &dst, options, ctl, Arc::clone(&copied_bytes)).await?;
        if !transfer.success {
            record_failure(ctl, &mut results, (&src, &dst), &transfer, options)?;
            continue;
        }

        let _ = preserve_metadata(&src, &dst, options);
        if options.verify_after_copy
            && !verify_hashes(
                (&src, &dst),
                size,
                (&transfer.src_hash, &transfer.dst_hash),
                options,
                ctl,
                &mut results,
            )?
        {
            continue;
        }
        if operation == TransferOperation::Move {
            let _ = std::fs::remove_file(&src);
        }
        ctl.file_completed(
            &mut results,
            FileTransferResult {
                src,
                size,
                src_hash: transfer.src_hash,
                dst_hash: transfer.dst_hash,
                verified: true,
                duration: file_start.elapsed(),
                replaced,
                dst_stamp: crate::fs::stamp::Stamp::of(&dst),
                dst,
            },
        );
    }

    if operation == TransferOperation::Move {
        let mut dirs = scan.dirs_to_delete;
        cleanup_source_dirs(&mut dirs, ctl);
    }
    Ok(results)
}

/// Records a file that could not be copied; with "halt on error" the job
/// fails.
fn record_failure(
    ctl: &JobControl,
    results: &mut TransferResults,
    (src, dst): (&std::path::Path, &std::path::Path),
    transfer: &single_file::TransferOutcome,
    options: &TransferOptions,
) -> anyhow::Result<()> {
    ctl.file_failed(
        results,
        FailedFile {
            src: src.to_path_buf(),
            dst: dst.to_path_buf(),
            error: transfer.last_error.clone(),
            retries: transfer.retries,
        },
    );
    if !options.halt_on_error {
        return Ok(());
    }
    ctl.emit(TransferEvent::JobFailed {
        job_id: ctl.job_id,
        error: format!(
            "Halt on error triggered by file failure: {}",
            transfer.last_error
        ),
    });
    Err(anyhow!("Halt on error: {}", transfer.last_error))
}
