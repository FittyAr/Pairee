//! Post-copy hash verification and source cleanup routines.

use super::super::super::control::JobControl;
use super::super::super::events::TransferEvent;
use super::super::super::job::{FailedFile, TransferResults};
use super::super::super::options::TransferOptions;
use super::super::fs_helpers::{forget_description, remove_forcing};
use anyhow::anyhow;
use std::path::{Path, PathBuf};

/// Compares the hashes computed while copying `src` → `dst`. Returns
/// `Ok(false)` (after recording the failure) on a mismatch.
pub fn verify_hashes(
    (src, dst): (&Path, &Path),
    size: u64,
    (src_hash, dst_hash): (&Option<String>, &Option<String>),
    options: &TransferOptions,
    ctl: &JobControl,
    results: &mut TransferResults,
) -> Result<bool, anyhow::Error> {
    let job_id = ctl.job_id;
    ctl.emit(TransferEvent::VerifyStarted {
        job_id,
        file: src.to_path_buf(),
        algorithm: options.hash_algorithm.as_str().to_string(),
    });

    if let (Some(sh), Some(dh)) = (src_hash.as_ref(), dst_hash.as_ref()) {
        ctl.emit(TransferEvent::VerifyProgress {
            job_id,
            bytes_verified: size,
            bytes_total: size,
        });

        if sh != dh {
            ctl.file_failed(
                results,
                FailedFile {
                    src: src.to_path_buf(),
                    dst: dst.to_path_buf(),
                    error: "Hash verification mismatch".to_string(),
                    retries: 0,
                },
            );
            if options.halt_on_error {
                ctl.emit(TransferEvent::JobFailed {
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

/// After a move: removes the emptied source folders (deepest first).
pub fn cleanup_source_dirs(dirs_to_delete: &mut [PathBuf], ctl: &JobControl) {
    dirs_to_delete.sort_by_key(|p| std::cmp::Reverse(p.as_os_str().len()));
    for dir in dirs_to_delete.iter() {
        if ctl.is_cancelled() {
            break;
        }
        forget_description(dir);
        let _ = remove_forcing(dir, |p| std::fs::remove_dir(p));
    }
}
