use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Creates a new directory at the specified path.
pub fn create_directory(path: &Path) -> Result<()> {
    fs::create_dir(path).context("Failed to create directory")
}

/// Renames or moves a file/directory synchronously.
/// On cross-device moves (different filesystems), falls back to copy + delete.
pub fn rename_or_move_sync(src: &Path, dst: &Path) -> Result<()> {
    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Cross-device link error: fall back to copy + delete
            let is_cross_device = e
                .raw_os_error()
                .map(|code| {
                    // EXDEV = 18 on Linux/macOS; ERROR_NOT_SAME_DEVICE = 17 on Windows
                    code == 18 || code == 17
                })
                .unwrap_or(false);

            if is_cross_device || e.kind() == std::io::ErrorKind::CrossesDevices {
                move_with_fallback(src, dst)
            } else {
                Err(e).context("Failed to rename/move item")
            }
        }
    }
}

/// Copies a file or directory tree to `dst`, then removes the source.
/// Used as a fallback when rename fails across filesystem boundaries.
pub fn move_with_fallback(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        copy_dir_recursive(src, dst)?;
        fs::remove_dir_all(src).context("Failed to remove source directory after cross-device move")
    } else {
        fs::copy(src, dst).context("Failed to copy file for cross-device move")?;
        fs::remove_file(src).context("Failed to remove source file after cross-device move")
    }
}

/// Recursively copies a directory and all its contents.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).context("Failed to create destination directory")?;
    for entry in fs::read_dir(src).context("Failed to read source directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).context("Failed to copy file in directory")?;
        }
    }
    Ok(())
}

/// Deletes a file or directory recursively.
pub fn delete_sync(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path).context("Failed to delete directory recursively")
    } else {
        if let (Some(parent), Some(filename)) = (path.parent(), path.file_name()) {
            if let Some(filename_str) = filename.to_str() {
                let _ = crate::fs::descriptions::remove_description(parent, filename_str);
            }
        }
        fs::remove_file(path).context("Failed to delete file")
    }
}
