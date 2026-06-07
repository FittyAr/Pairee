use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Creates a new directory at the specified path.
pub fn create_directory(path: &Path) -> Result<()> {
    fs::create_dir(path).context("Failed to create directory")
}

/// Renames or moves a file/directory synchronously.
pub fn rename_or_move_sync(src: &Path, dst: &Path) -> Result<()> {
    fs::rename(src, dst).context("Failed to rename/move item")
}

/// Deletes a file or directory recursively.
pub fn delete_sync(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path).context("Failed to delete directory recursively")
    } else {
        fs::remove_file(path).context("Failed to delete file")
    }
}
