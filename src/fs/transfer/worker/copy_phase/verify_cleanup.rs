//! Post-copy hash verification and source cleanup routines.

use super::super::super::events::TransferEvent;
use super::super::super::job::{FailedFile, TransferResults};
use super::super::super::options::TransferOptions;
use super::super::fs_helpers::make_writable_helper;
use anyhow::anyhow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use uuid::Uuid;

pub fn verify_hashes(
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
                return Err(anyhow!("Halt on error: Hash mismatch"));
            }
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn cleanup_source_dirs(dirs_to_delete: &mut [PathBuf], is_cancelled: &AtomicBool) {
    dirs_to_delete.sort_by_key(|p| std::cmp::Reverse(p.as_os_str().len()));
    for dir in dirs_to_delete.iter() {
        if is_cancelled.load(Ordering::Relaxed) {
            break;
        }
        if let (Some(parent), Some(filename)) = (dir.parent(), dir.file_name())
            && let Some(filename_str) = filename.to_str()
        {
            let _ = crate::fs::descriptions::remove_description(parent, filename_str);
        }
        if std::fs::remove_dir(dir).is_err() {
            let _ = make_writable_helper(dir);
            let _ = std::fs::remove_dir(dir);
        }
    }
}
