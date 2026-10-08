use crate::fs::entry::FileEntry;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

mod admin;
mod sort;

pub use sort::sort_entries;

#[allow(dead_code)]
pub fn read_directory(
    path: &Path,
    show_hidden: bool,
    case_sensitive_sort: bool,
    treat_digits_as_numbers: bool,
    _sorting_collation: &str,
    req_admin_reading: bool,
) -> Result<Vec<FileEntry>> {
    read_directory_ext(
        path,
        show_hidden,
        case_sensitive_sort,
        treat_digits_as_numbers,
        _sorting_collation,
        req_admin_reading,
        crate::app::state::SortField::Name,
        false,
        false,
        true,
    )
}

pub fn read_directory_ext(
    path: &Path,
    show_hidden: bool,
    case_sensitive_sort: bool,
    treat_digits_as_numbers: bool,
    _sorting_collation: &str,
    req_admin_reading: bool,
    sort_field: crate::app::state::SortField,
    sort_reverse: bool,
    sort_folder_names_by_extension: bool,
    show_dotdot_in_root_folders: bool,
) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    // 1. Add ".." parent directory entry
    //    Always added if a parent exists; or if show_dotdot_in_root_folders is enabled.
    let has_parent = path.parent().is_some();
    if has_parent {
        let parent = path.parent().unwrap();
        entries.push(FileEntry {
            name: "..".to_string(),
            path: parent.to_path_buf(),
            size: 0,
            is_dir: true,
            is_symlink: false,
            modified: None,
        });
    } else if show_dotdot_in_root_folders {
        // Insert a ".." that stays in the current root (navigating up from root stays at root).
        entries.push(FileEntry {
            name: "..".to_string(),
            path: path.to_path_buf(),
            size: 0,
            is_dir: true,
            is_symlink: false,
            modified: None,
        });
    }

    // 2. Read directory contents
    let read_res = fs::read_dir(path);
    let read_entries = match read_res {
        Ok(read_dir) => {
            let mut items = Vec::new();
            for entry in read_dir.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                // `DirEntry::metadata` does not follow symlinks: keep it for the
                // link flag and the hidden attribute, but follow the link so a
                // symlinked directory is still listed (and entered) as a directory.
                let link_metadata = entry.metadata().ok();

                // Skip hidden files if show_hidden is not enabled
                if !show_hidden && is_hidden(&name, link_metadata.as_ref()) {
                    continue;
                }

                let is_symlink = link_metadata
                    .as_ref()
                    .is_some_and(|m| m.file_type().is_symlink());
                let metadata = if is_symlink {
                    fs::metadata(entry.path()).ok().or(link_metadata)
                } else {
                    link_metadata
                };
                let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata.and_then(|m| m.modified().ok());

                items.push(FileEntry {
                    name,
                    path: entry.path(),
                    size,
                    is_dir,
                    is_symlink,
                    modified,
                });
            }
            Ok(items)
        }
        Err(e) => {
            if req_admin_reading {
                admin::read_directory_as_admin(path)
            } else {
                Err(anyhow::anyhow!(e))
            }
        }
    };

    let mut read_entries = read_entries.context(format!("Failed to read directory: {:?}", path))?;
    entries.append(&mut read_entries);

    // 3. Sort entries using the extracted public function
    sort::sort_entries(
        &mut entries,
        sort_field,
        sort_reverse,
        case_sensitive_sort,
        treat_digits_as_numbers,
        sort_folder_names_by_extension,
    );

    Ok(entries)
}

/// Dot-files are hidden everywhere; on Windows the hidden attribute counts too.
fn is_hidden(name: &str, metadata: Option<&fs::Metadata>) -> bool {
    if name.starts_with('.') {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        if let Some(m) = metadata {
            return m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0;
        }
    }
    #[cfg(not(windows))]
    let _ = metadata;
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(dir: &Path, show_hidden: bool) -> Vec<FileEntry> {
        read_directory(dir, show_hidden, false, false, "", false)
            .unwrap()
            .into_iter()
            .filter(|e| e.name != "..")
            .collect()
    }

    #[test]
    fn dot_files_respect_show_hidden() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".secret"), b"x").unwrap();
        std::fs::write(dir.path().join("plain"), b"x").unwrap();
        assert_eq!(names(dir.path(), false).len(), 1);
        assert_eq!(names(dir.path(), true).len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_directory_is_listed_as_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("real")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("real"), dir.path().join("link")).unwrap();
        let link = names(dir.path(), true)
            .into_iter()
            .find(|e| e.name == "link")
            .unwrap();
        assert!(link.is_dir);
        assert!(link.is_symlink);
    }

    #[cfg(windows)]
    #[test]
    fn windows_hidden_attribute_respects_show_hidden() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("hidden.txt");
        std::fs::write(&file, b"x").unwrap();
        let status = std::process::Command::new("attrib")
            .arg("+h")
            .arg(&file)
            .status()
            .unwrap();
        assert!(status.success());
        assert!(names(dir.path(), false).is_empty());
        assert_eq!(names(dir.path(), true).len(), 1);
    }
}
