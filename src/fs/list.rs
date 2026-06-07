use super::entry::FileEntry;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Reads directory contents and returns a sorted list of FileEntry structs.
pub fn read_directory(path: &Path, show_hidden: bool) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    // 1. Add ".." parent directory entry if a parent exists
    if let Some(parent) = path.parent() {
        entries.push(FileEntry {
            name: "..".to_string(),
            path: parent.to_path_buf(),
            size: 0,
            is_dir: true,
            is_symlink: false,
            modified: None,
        });
    }

    // 2. Read contents of the directory
    let read_dir = fs::read_dir(path).context(format!("Failed to read directory: {:?}", path))?;

    for entry in read_dir.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();

        // Skip hidden files if not enabled in settings
        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let metadata = entry.metadata().ok();
        let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let is_symlink = metadata.as_ref().map(|m| m.is_symlink()).unwrap_or(false);
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified = metadata.and_then(|m| m.modified().ok());

        entries.push(FileEntry {
            name,
            path: entry.path(),
            size,
            is_dir,
            is_symlink,
            modified,
        });
    }

    // 3. Sort entries:
    //    - Pin ".." parent folder as first element
    //    - Directories come before files
    //    - Alphabetical sort (case-insensitive) within those categories
    entries.sort_by(|a, b| {
        if a.name == ".." {
            return std::cmp::Ordering::Less;
        }
        if b.name == ".." {
            return std::cmp::Ordering::Greater;
        }

        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}
