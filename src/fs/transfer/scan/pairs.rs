//! Scan of explicit `(source, destination)` pairs (undo / redo of copies and
//! moves). Missing sources are left out; a folder pair (a folder moved by a
//! rename) is expanded into its files when it cannot be renamed back.

use super::{FileMapping, ScanOutcome};
use crate::fs::transfer::control::JobControl;
use crate::fs::transfer::job::FailedFile;
use crate::fs::vfs::LocalVfs;
use crate::fs::walk::{WalkOptions, Walker};
use std::path::{Path, PathBuf};

/// Mappings and destination folders of `pairs` (local only).
pub fn scan_pairs(pairs: &[(PathBuf, PathBuf)], ctl: &JobControl) -> ScanOutcome {
    let mut out = ScanOutcome::default();
    for (from, to) in pairs {
        let Ok(meta) = from.symlink_metadata() else {
            continue;
        };
        if meta.is_dir() {
            folder_pair(from, to, &mut out);
            continue;
        }
        if let Some(parent) = to.parent() {
            out.dst_dirs.push(parent.to_path_buf());
        }
        let size = if meta.file_type().is_symlink() {
            0
        } else {
            meta.len()
        };
        push(&mut out, (from.clone(), to.clone(), size));
    }
    ctl.scan_complete(out.mappings.len(), out.total_bytes);
    out
}

fn push(out: &mut ScanOutcome, mapping: FileMapping) {
    out.total_bytes += mapping.2;
    out.mappings.push(mapping);
}

/// Every file of the folder `from`, placed below `to`; the emptied source
/// folders are removed after a move.
fn folder_pair(from: &Path, to: &Path, out: &mut ScanOutcome) {
    out.dst_dirs.push(to.to_path_buf());
    out.dirs_to_delete.push(from.to_path_buf());
    for item in Walker::new(&LocalVfs, from, WalkOptions::default()) {
        let item = match item {
            Ok(item) => item,
            Err(e) => {
                out.refused.push(FailedFile {
                    src: e.path().map(Path::to_path_buf).unwrap_or_default(),
                    dst: PathBuf::new(),
                    error: e.to_string(),
                    retries: 0,
                });
                continue;
            }
        };
        let Ok(rel) = item.path().strip_prefix(from) else {
            continue;
        };
        let dst = to.join(rel);
        if item.descends {
            out.dirs_to_delete.push(item.entry.path);
            out.dst_dirs.push(dst);
        } else {
            let size = if item.is_kept_link() {
                0
            } else {
                item.entry.size
            };
            push(out, (item.entry.path, dst, size));
        }
    }
}
