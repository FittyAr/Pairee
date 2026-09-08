//! Remote SFTP filesystem operations (read, delete, walk, rename, create dir).

use crate::app::state::SortField;
use crate::config::localization::t;
use crate::fs::entry::FileEntry;
use anyhow::Result;
use ssh2::Sftp;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub fn read_directory(
    sftp: &Sftp,
    path: &Path,
    show_hidden: bool,
    case_sensitive_sort: bool,
    treat_digits_as_numbers: bool,
    sort_field: SortField,
    sort_reverse: bool,
    show_dotdot_in_root_folders: bool,
) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    // 1. Add ".." parent directory entry
    let path_str = path.to_string_lossy().to_string();
    let is_root = path_str == "/" || path_str.is_empty();
    if !is_root {
        let parent = path.parent().unwrap_or(Path::new("/"));
        entries.push(FileEntry {
            name: "..".to_string(),
            path: parent.to_path_buf(),
            size: 0,
            is_dir: true,
            is_symlink: false,
            modified: None,
        });
    } else if show_dotdot_in_root_folders {
        entries.push(FileEntry {
            name: "..".to_string(),
            path: path.to_path_buf(),
            size: 0,
            is_dir: true,
            is_symlink: false,
            modified: None,
        });
    }

    // 2. Read SFTP directory contents
    let read_res = sftp.readdir(path);
    let mut read_entries = match read_res {
        Ok(items) => {
            let mut mapped = Vec::new();
            for (path_buf, stat) in items {
                let name = path_buf
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();

                if name.is_empty() || name == "." || name == ".." {
                    continue;
                }

                if !show_hidden && name.starts_with('.') {
                    continue;
                }

                let is_dir = stat.is_dir();
                let is_symlink = stat.file_type().is_symlink();
                let size = stat.size.unwrap_or(0);
                let modified = stat
                    .mtime
                    .map(|mtime| SystemTime::UNIX_EPOCH + Duration::from_secs(mtime));

                mapped.push(FileEntry {
                    name,
                    path: path_buf,
                    size,
                    is_dir,
                    is_symlink,
                    modified,
                });
            }
            mapped
        }
        Err(e) => anyhow::bail!(t("error_ssh_read_dir_failed").replace("{}", &e.to_string())),
    };

    entries.append(&mut read_entries);

    // 3. Sort entries (pinning ".." first) using the centralized sort_entries helper
    crate::fs::list::sort_entries(
        &mut entries,
        sort_field,
        sort_reverse,
        case_sensitive_sort,
        treat_digits_as_numbers,
        false,
    );

    Ok(entries)
}

pub fn delete_recursive(sftp: &Sftp, path: &Path) -> Result<()> {
    let mut stack: Vec<PathBuf> = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        let (is_dir, children) = match sftp.stat(&current) {
            Ok(stat) => {
                if stat.is_dir() {
                    let kids = sftp.readdir(&current)?;
                    let mut names: Vec<PathBuf> = Vec::with_capacity(kids.len());
                    for (entry_path, entry_stat) in kids {
                        let name = entry_path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        if name == "." || name == ".." || name.is_empty() {
                            continue;
                        }
                        if entry_stat.is_dir() {
                            stack.push(entry_path);
                        } else {
                            names.push(entry_path);
                        }
                    }
                    (true, Some(names))
                } else {
                    (false, None)
                }
            }
            Err(_) => (false, None),
        };

        if is_dir {
            if let Some(kids) = children {
                for k in kids {
                    sftp.unlink(&k)?;
                }
            }
            sftp.rmdir(&current)?;
        } else {
            let _ = sftp.unlink(&current);
        }
    }
    Ok(())
}

pub fn walk_dir(sftp: &Sftp, root: &Path) -> Result<Vec<(PathBuf, bool, u64)>> {
    let mut results = Vec::new();
    let mut to_visit = vec![root.to_path_buf()];

    while let Some(dir) = to_visit.pop() {
        if let Ok(entries) = sftp.readdir(&dir) {
            for (path_buf, stat) in entries {
                let name = path_buf
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if name == "." || name == ".." || name.is_empty() {
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
