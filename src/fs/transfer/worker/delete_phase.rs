use anyhow::anyhow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use super::super::control::JobControl;
use super::super::job::{FailedFile, FileTransferResult, TransferResults};
use super::super::options::TransferOptions;
use super::fs_helpers::{forget_description, remove_forcing, send_to_recycle_bin_helper};
use super::scan::ScanOutcome;
use super::speed::spawn_speed_reporter;

/// Run the delete phase (recycle bin or permanent delete) and return results.
pub(super) async fn run_delete_phase(
    sources: &[PathBuf],
    scan: ScanOutcome,
    options: &TransferOptions,
    ctl: &JobControl,
) -> Result<TransferResults, anyhow::Error> {
    let mut results = TransferResults::default();
    let deleted_bytes = Arc::new(AtomicU64::new(0));
    let _speed_reporter = spawn_speed_reporter(ctl, Arc::clone(&deleted_bytes), scan.total_bytes);

    if options.delete_to_recycle_bin {
        for (idx, src) in sources.iter().enumerate() {
            ctl.ensure_running()?;
            // Measure before trashing: the path is gone afterwards.
            let size = src.symlink_metadata().map(|m| m.len()).unwrap_or(0);
            let deleted = delete_one(ctl, &mut results, (idx, src), size, |p| {
                send_to_recycle_bin_helper(p).map_err(|e| e.to_string())
            });
            match deleted {
                Ok(size) => deleted_bytes.fetch_add(size, Ordering::SeqCst),
                Err(()) if options.halt_on_error => {
                    return Err(anyhow!("Halt on error: Recycle Bin deletion failed"));
                }
                Err(()) => 0,
            };
        }
        return ctl.job_completed(results);
    }

    for (idx, (src, _, size)) in scan.mappings.into_iter().enumerate() {
        ctl.wait_if_paused().await?;
        if ctl.take_user_skip(&src, &mut results) {
            continue;
        }
        let deleted = delete_one(ctl, &mut results, (idx, &src), size, |p| {
            remove_forcing(p, |p| std::fs::remove_file(p)).map_err(|e| e.to_string())
        });
        match deleted {
            Ok(size) => deleted_bytes.fetch_add(size, Ordering::SeqCst),
            Err(()) if options.halt_on_error => {
                return Err(anyhow!("Halt on error: Deletion failed"));
            }
            Err(()) => 0,
        };
    }

    let mut dirs = scan.dirs_to_delete;
    dirs.sort_by_key(|p| std::cmp::Reverse(p.as_os_str().len()));
    for dir in dirs {
        forget_description(&dir);
        if let Err(e) = remove_forcing(&dir, |p| std::fs::remove_dir(p)) {
            ctl.file_failed(&mut results, failed(&dir, e.to_string()));
        }
    }
    ctl.job_completed(results)
}

/// Deletes one entry (file `index`) with `remove`, recording the outcome.
/// Returns the deleted size, or `Err` after recording the failure.
fn delete_one(
    ctl: &JobControl,
    results: &mut TransferResults,
    (index, src): (usize, &Path),
    size: u64,
    remove: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<u64, ()> {
    let start = Instant::now();
    ctl.file_started(src, index);
    forget_description(src);
    match remove(src) {
        Err(error) => {
            ctl.file_failed(results, failed(src, error));
            Err(())
        }
        Ok(()) => {
            ctl.file_completed(
                results,
                FileTransferResult {
                    src: src.to_path_buf(),
                    dst: PathBuf::new(),
                    size,
                    src_hash: None,
                    dst_hash: None,
                    verified: true,
                    duration: start.elapsed(),
                },
            );
            Ok(size)
        }
    }
}

fn failed(src: &Path, error: String) -> FailedFile {
    FailedFile {
        src: src.to_path_buf(),
        dst: PathBuf::new(),
        error,
        retries: 0,
    }
}
