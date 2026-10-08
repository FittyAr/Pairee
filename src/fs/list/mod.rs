use crate::fs::entry::FileEntry;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

mod admin;
mod sort;

pub use sort::sort_entries;

/// Sorting / visibility options of a directory listing (local or remote).
#[derive(Debug, Clone)]
pub struct ListOptions {
    pub show_hidden: bool,
    pub case_sensitive: bool,
    /// "Natural" collation: digit runs compare as numbers.
    pub natural: bool,
    /// Retry with elevated rights when reading fails.
    pub req_admin: bool,
    pub sort_field: crate::app::state::SortField,
    pub sort_reverse: bool,
    /// Sort folders by extension too.
    pub folder_by_ext: bool,
    /// Show `..` in root folders (pointing at the root itself).
    pub show_dotdot: bool,
}

impl ListOptions {
    /// Sorts `entries` (pinning `..` first) as these options say.
    pub fn sort(&self, entries: &mut Vec<FileEntry>) {
        sort::sort_entries(
            entries,
            self.sort_field,
            self.sort_reverse,
            self.case_sensitive,
            self.natural,
            self.folder_by_ext,
        );
    }
}

/// Lists `path` sorted as `opts` say.
pub fn read_directory_ext(path: &Path, opts: &ListOptions) -> Result<Vec<FileEntry>> {
    let show_hidden = opts.show_hidden;
    let show_dotdot_in_root_folders = opts.show_dotdot;
    let req_admin_reading = opts.req_admin;
    let mut entries = Vec::new();

    // 1. Add ".." parent directory entry
    //    Always added if a parent exists; or if show_dotdot_in_root_folders is enabled.
    if let Some(parent) = path.parent() {
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

    // 3. Sort entries
    opts.sort(&mut entries);

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
        read_directory_ext(
            dir,
            &ListOptions {
                show_hidden,
                case_sensitive: false,
                natural: false,
                req_admin: false,
                sort_field: crate::app::state::SortField::Name,
                sort_reverse: false,
                folder_by_ext: false,
                show_dotdot: true,
            },
        )
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
