//! Moves within one volume are renames: a folder of any size moves
//! instantly, with nothing copied and nothing to verify. A source is only
//! renamed when its target is free (conflicts go through the normal
//! per-file path) and the job has no filter or link handling that a rename
//! would bypass; when the rename fails — another volume
//! (`EXDEV` / `ERROR_NOT_SAME_DEVICE`) or anything else — the source is
//! moved by copy + delete as before.
//!
//! As the transfer design asks (§2.6), a move with "verify after copy"
//! always copies, verifies and deletes.

use super::destination::ensure_dir;
use crate::fs::stamp::Stamp;
use crate::fs::transfer::control::{JobControl, done};
use crate::fs::transfer::job::TransferResults;
use crate::fs::transfer::options::TransferOptions;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Renames the roots that can be renamed (recording them as done) and
/// returns the others, to be moved by copy + delete.
pub(super) fn rename_roots(
    roots: Vec<(PathBuf, PathBuf)>,
    options: &TransferOptions,
    ctl: &JobControl,
    results: &mut TransferResults,
) -> Vec<(PathBuf, PathBuf)> {
    if !allowed(options) {
        return roots;
    }
    let mut rest = Vec::new();
    for (index, (src, dst)) in roots.into_iter().enumerate() {
        if ctl.is_cancelled() || !try_rename(&src, &dst, &mut results.created_dirs) {
            rest.push((src, dst));
            continue;
        }
        let size =
            std::fs::symlink_metadata(&dst).map_or(0, |m| if m.is_dir() { 0 } else { m.len() });
        ctl.file_started(&src, index);
        let mut result = done(&src, dst.clone(), size, Instant::now());
        result.dst_stamp = Stamp::of(&dst);
        ctl.file_completed(results, result);
    }
    rest
}

/// A rename gives the same result as the copy + delete it replaces.
fn allowed(options: &TransferOptions) -> bool {
    options.filter_mask.as_deref().is_none_or(str::is_empty)
        && !options.follow_symlinks
        && !options.skip_symlinks
        && !options.verify_after_copy
}

/// Renames `src` to the free path `dst`; `false` when that is not possible.
fn try_rename(src: &Path, dst: &Path, created: &mut Vec<PathBuf>) -> bool {
    if std::fs::symlink_metadata(src).is_err() {
        return false;
    }
    let free = match std::fs::symlink_metadata(dst) {
        Err(e) => e.kind() == std::io::ErrorKind::NotFound,
        Ok(_) => is_case_change(src, dst),
    };
    if !free {
        return false;
    }
    if let Some(parent) = dst.parent() {
        ensure_dir(parent, created);
    }
    match std::fs::rename(src, dst) {
        Ok(()) => true,
        Err(e) => {
            log::debug!("fast move of {src:?} not possible ({e}); copying instead");
            false
        }
    }
}

/// `dst` only changes the letter case of `src`'s name, on a filesystem that
/// ignores case (so `dst` "exists": it is `src` itself).
fn is_case_change(src: &Path, dst: &Path) -> bool {
    let (Some(a), Some(b)) = (src.file_name(), dst.file_name()) else {
        return false;
    };
    a != b
        && a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
        && src.parent() == dst.parent()
        && crate::fs::transfer::scan::guard::same_entry(&crate::fs::vfs::LocalVfs, src, dst)
}
