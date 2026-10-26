//! Delete phase: every scanned entry is removed (links themselves, never
//! what they point to), then the emptied folders, deepest first — or the
//! sources go to the trash as a whole. With a filter mask only the entries
//! it selects go (to the trash or for good), and a folder that still holds
//! what the mask left out stays.

use anyhow::anyhow;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use super::super::control::{JobControl, done, failed_io};
use super::super::job::TransferResults;
use super::super::options::TransferOptions;
use super::super::scan::ScanOutcome;
use super::super::speed::SpeedReporter;
use super::fs_helpers::send_to_recycle_bin_helper;
use crate::fs::descriptions::forget_description;
use crate::fs::remove::remove_entry;

/// Run the delete phase (recycle bin or permanent delete) and return results
/// (the caller reports the end of the job).
pub(super) async fn run_delete_phase(
    sources: &[PathBuf],
    mut scan: ScanOutcome,
    options: &TransferOptions,
    ctl: &JobControl,
) -> Result<TransferResults, anyhow::Error> {
    let mut results = TransferResults::default();
    let deleted_bytes = Arc::new(AtomicU64::new(0));
    let _speed = SpeedReporter::spawn(ctl, Arc::clone(&deleted_bytes), scan.total_bytes);

    let filtered = options
        .filter_mask
        .as_deref()
        .is_some_and(|mask| !mask.trim().is_empty());
    if options.delete_to_recycle_bin && !filtered {
        for (idx, src) in sources.iter().enumerate() {
            ctl.ensure_running()?;
            // Measure before trashing: the path is gone afterwards.
            let size = src.symlink_metadata().map(|m| m.len()).unwrap_or(0);
            let deleted = delete_one(
                ctl,
                &mut results,
                (idx, src),
                size,
                send_to_recycle_bin_helper,
            );
            match deleted {
                Ok(size) => deleted_bytes.fetch_add(size, Ordering::SeqCst),
                Err(()) if options.halt_on_error => {
                    return Err(anyhow!("Halt on error: Recycle Bin deletion failed"));
                }
                Err(()) => 0,
            };
        }
        return Ok(results);
    }

    refuse_held_sources(sources, &mut scan, ctl, &mut results);
    for failure in scan.refused {
        ctl.file_failed(&mut results, failure);
    }
    for (idx, (src, _, size)) in scan.mappings.into_iter().enumerate() {
        ctl.wait_if_paused().await?;
        if ctl.take_user_skip(&src, &mut results) {
            continue;
        }
        let deleted = delete_one(ctl, &mut results, (idx, &src), size, |p| {
            if options.delete_to_recycle_bin {
                send_to_recycle_bin_helper(p)
            } else {
                remove_entry(p)
            }
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
        match remove_entry(&dir) {
            Ok(()) => forget_description(&dir),
            // What the mask left out stays, and so does its folder.
            Err(_) if filtered && holds_entries(&dir) => {}
            // Kept because something in it was not deleted, which is
            // already reported with its reason.
            Err(e) if e.kind() == ErrorKind::DirectoryNotEmpty && failed_below(&results, &dir) => {}
            Err(e) => ctl.file_failed(&mut results, failed_io(&dir, PathBuf::new(), &e)),
        }
    }
    Ok(results)
}

/// Reports each source whose folder does not let it be removed, and drops
/// everything below it from `scan`: emptying a folder that then stays
/// would only lose what it held.
fn refuse_held_sources(
    sources: &[PathBuf],
    scan: &mut ScanOutcome,
    ctl: &JobControl,
    results: &mut TransferResults,
) {
    for src in sources {
        let Some(error) = removal_refused(src) else {
            continue;
        };
        ctl.file_failed(results, failed_io(src, PathBuf::new(), &error));
        scan.mappings.retain(|(path, _, _)| !path.starts_with(src));
        scan.dirs_to_delete.retain(|path| !path.starts_with(src));
        scan.refused.retain(|failure| !failure.src.starts_with(src));
    }
}

/// Unix: the error when the folder holding `path` refuses this process the
/// write and search permission that removing an entry from it takes.
#[cfg(unix)]
fn removal_refused(path: &Path) -> Option<std::io::Error> {
    use std::os::unix::ffi::OsStrExt;
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty())?;
    let parent = std::ffi::CString::new(parent.as_os_str().as_bytes()).ok()?;
    // SAFETY: `parent` is a valid NUL-terminated path for the call.
    let rc = unsafe {
        libc::faccessat(
            libc::AT_FDCWD,
            parent.as_ptr(),
            libc::W_OK | libc::X_OK,
            libc::AT_EACCESS,
        )
    };
    let error = std::io::Error::last_os_error();
    (rc != 0 && error.kind() == ErrorKind::PermissionDenied).then_some(error)
}

/// Elsewhere the removal itself reports a refusal.
#[cfg(not(unix))]
fn removal_refused(_path: &Path) -> Option<std::io::Error> {
    None
}

/// A failure was recorded for `dir` (unreadable) or something inside it.
fn failed_below(results: &TransferResults, dir: &Path) -> bool {
    results.failed_files.iter().any(|f| f.src.starts_with(dir))
}

/// `dir` is a folder with something in it.
fn holds_entries(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some())
}

/// Deletes one entry (file `index`) with `remove`, recording the outcome.
/// Returns the deleted size, or `Err` after recording the failure.
fn delete_one(
    ctl: &JobControl,
    results: &mut TransferResults,
    (index, src): (usize, &Path),
    size: u64,
    remove: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<u64, ()> {
    let start = Instant::now();
    ctl.file_started(src, index);
    match remove(src) {
        Err(error) => {
            ctl.file_failed(results, failed_io(src, PathBuf::new(), &error));
            Err(())
        }
        Ok(()) => {
            forget_description(src);
            ctl.file_completed(results, done(src, PathBuf::new(), size, start));
            Ok(size)
        }
    }
}
