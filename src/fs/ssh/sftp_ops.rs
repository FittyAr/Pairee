//! Mapping of SFTP `readdir` / `stat` results to [`VfsEntry`]s.

use crate::fs::attrs::FileAttrs;
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
        modified: stat_time(stat.mtime),
    })
}

/// An SFTP time stamp (seconds since the epoch) as a `SystemTime`.
fn stat_time(secs: Option<u64>) -> Option<SystemTime> {
    secs.map(|secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
}

/// Attributes dialog data of a remote entry. SFTP v3 reports numeric
/// owner and group only, shown as `uid:gid`, and no creation time.
pub(super) fn sftp_attrs(path: &Path, stat: &FileStat) -> FileAttrs {
    let mode = stat.perm.unwrap_or(0);
    let owner = match (stat.uid, stat.gid) {
        (Some(uid), Some(gid)) => format!("{uid}:{gid}"),
        (Some(uid), None) => uid.to_string(),
        _ => crate::config::localization::t("info_na"),
    };
    FileAttrs {
        path: path.to_path_buf(),
        mode,
        readonly: mode & 0o222 == 0,
        size: stat.size.unwrap_or(0),
        modified: stat_time(stat.mtime),
        created: None,
        owner,
        nlinks: 1,
    }
}
