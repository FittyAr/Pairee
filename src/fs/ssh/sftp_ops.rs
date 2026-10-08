//! Remote SFTP filesystem operations (read, delete, walk, rename, create dir).

use crate::config::localization::t;
use crate::fs::entry::FileEntry;
use crate::fs::list::ListOptions;
use anyhow::Result;
use ssh2::{FileStat, Sftp};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Lists remote `path` sorted as `opts` say (folders are never sorted by
/// extension remotely).
pub fn read_directory(sftp: &Sftp, path: &Path, opts: &ListOptions) -> Result<Vec<FileEntry>> {
    let show_hidden = opts.show_hidden;
    let mut entries: Vec<FileEntry> = parent_entry(path, opts.show_dotdot).into_iter().collect();

    // Read SFTP directory contents
    let mut read_entries = match sftp.readdir(path) {
        Ok(items) => items
            .into_iter()
            .filter_map(|(path_buf, stat)| map_entry(path_buf, &stat, show_hidden))
            .collect::<Vec<_>>(),
        Err(e) => anyhow::bail!(t("error_ssh_read_dir_failed").replace("{}", &e.to_string())),
    };

    entries.append(&mut read_entries);

    // Sort entries (pinning ".." first)
    ListOptions {
        folder_by_ext: false,
        ..opts.clone()
    }
    .sort(&mut entries);

    Ok(entries)
}

/// The `..` entry shown at the top of a remote listing, if any.
///
/// Non-root folders always get one pointing at the parent; the root only
/// gets one (pointing at itself) when `show_dotdot_in_root_folders` is set.
pub(super) fn parent_entry(path: &Path, show_dotdot_in_root_folders: bool) -> Option<FileEntry> {
    let path_str = path.to_string_lossy();
    let is_root = path_str == "/" || path_str.is_empty();
    let target = if !is_root {
        path.parent().unwrap_or(Path::new("/"))
    } else if show_dotdot_in_root_folders {
        path
    } else {
        return None;
    };
    Some(FileEntry {
        name: "..".to_string(),
        path: target.to_path_buf(),
        size: 0,
        is_dir: true,
        is_symlink: false,
        modified: None,
    })
}

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

/// Maps one SFTP `readdir` item to a panel entry, applying the hidden filter.
pub(super) fn map_entry(
    path_buf: PathBuf,
    stat: &FileStat,
    show_hidden: bool,
) -> Option<FileEntry> {
    let name = entry_name(&path_buf);
    if !is_real_child(&name) || (!show_hidden && name.starts_with('.')) {
        return None;
    }
    Some(FileEntry {
        name,
        path: path_buf,
        size: stat.size.unwrap_or(0),
        is_dir: stat.is_dir(),
        is_symlink: stat.file_type().is_symlink(),
        modified: stat
            .mtime
            .map(|mtime| SystemTime::UNIX_EPOCH + Duration::from_secs(mtime)),
    })
}

pub fn walk_dir(sftp: &Sftp, root: &Path) -> Result<Vec<(PathBuf, bool, u64)>> {
    let mut results = Vec::new();
    let mut to_visit = vec![root.to_path_buf()];

    while let Some(dir) = to_visit.pop() {
        if let Ok(entries) = sftp.readdir(&dir) {
            for (path_buf, stat) in entries {
                if !is_real_child(&entry_name(&path_buf)) {
                    continue;
                }
                let is_dir = stat.is_dir();
                let size = stat.size.unwrap_or(0);
                results.push((path_buf.clone(), is_dir, size));
                if is_dir {
                    to_visit.push(path_buf);
                }
            }
        }
    }
    Ok(results)
}
