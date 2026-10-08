//! Mapping of SFTP `readdir` / `stat` results to [`VfsEntry`]s.

use crate::fs::vfs::VfsEntry;
use ssh2::FileStat;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Final path component as a display name (empty when there is none).
pub(super) fn entry_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `readdir` may report `.`/`..` or nameless entries; those are skipped.
pub(super) fn is_real_child(name: &str) -> bool {
    !(name.is_empty() || name == "." || name == "..")
}

/// Maps one SFTP item to an entry (`None` for `.`, `..` and nameless items).
/// `readdir` and `lstat` report link attributes, so links are not followed.
pub(super) fn sftp_entry(path: PathBuf, stat: &FileStat) -> Option<VfsEntry> {
    let name = entry_name(&path);
    if !is_real_child(&name) {
        return None;
    }
    Some(VfsEntry {
        hidden: name.starts_with('.'),
        name,
        path,
        size: stat.size.unwrap_or(0),
        is_dir: stat.is_dir(),
        is_symlink: stat.file_type().is_symlink(),
        modified: stat
            .mtime
            .map(|mtime| SystemTime::UNIX_EPOCH + Duration::from_secs(mtime)),
    })
}
