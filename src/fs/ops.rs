use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

#[cfg(target_os = "windows")]
fn send_to_recycle_bin(path: &Path) -> Result<()> {
    use std::process::Command;
    let path_str = path.to_string_lossy().replace('\'', "''");
    let ps_cmd = if path.is_dir() {
        format!(
            "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory('{}', 'OnlyErrorDialogs', 'SendToRecycleBin')",
            path_str
        )
    } else {
        format!(
            "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile('{}', 'OnlyErrorDialogs', 'SendToRecycleBin')",
            path_str
        )
    };
    let output = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_cmd])
        .output()
        .context("Failed to execute PowerShell trash command")?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("PowerShell Recycle Bin error: {}", err);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn send_to_recycle_bin(path: &Path) -> Result<()> {
    use std::process::Command;
    let status = Command::new("gio").arg("trash").arg(path).status();
    if let Ok(s) = status {
        if s.success() {
            return Ok(());
        }
    }
    let status = Command::new("trash-put").arg(path).status();
    if let Ok(s) = status {
        if s.success() {
            return Ok(());
        }
    }
    // Fallback to standard delete if trash command fails
    if path.is_dir() {
        delete_dir_recursive(path)
    } else {
        fs::remove_file(path).context("Failed to delete file")
    }
}

#[cfg(target_os = "windows")]
fn run_as_admin_delete(path: &Path) -> Result<()> {
    use std::process::Command;
    let path_str = path.to_string_lossy().replace('"', "\\\"");
    let ps_arg = format!(
        "Start-Process powershell -ArgumentList '-NoProfile -Command Remove-Item -Path \\\"{}\\\" -Force -Recurse' -Verb RunAs -WindowStyle Hidden",
        path_str
    );
    let status = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_arg])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to delete as administrator")
    }
}

#[cfg(not(target_os = "windows"))]
fn run_as_admin_delete(path: &Path) -> Result<()> {
    use std::process::Command;
    let status = Command::new("sudo")
        .arg("rm")
        .arg("-rf")
        .arg(path)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to delete as administrator via sudo")
    }
}

#[cfg(target_os = "windows")]
fn run_as_admin_mkdir(path: &Path) -> Result<()> {
    use std::process::Command;
    let path_str = path.to_string_lossy().replace('"', "\\\"");
    let ps_arg = format!(
        "Start-Process powershell -ArgumentList '-NoProfile -Command New-Item -ItemType Directory -Path \\\"{}\\\" -Force' -Verb RunAs -WindowStyle Hidden",
        path_str
    );
    let status = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_arg])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to create folder as administrator")
    }
}

#[cfg(not(target_os = "windows"))]
fn run_as_admin_mkdir(path: &Path) -> Result<()> {
    use std::process::Command;
    let status = Command::new("sudo")
        .arg("mkdir")
        .arg("-p")
        .arg(path)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to create folder as administrator via sudo")
    }
}

#[cfg(target_os = "windows")]
fn run_as_admin_rename(src: &Path, dst: &Path) -> Result<()> {
    use std::process::Command;
    let src_str = src.to_string_lossy().replace('"', "\\\"");
    let dst_str = dst.to_string_lossy().replace('"', "\\\"");
    let ps_arg = format!(
        "Start-Process powershell -ArgumentList '-NoProfile -Command Move-Item -Path \\\"{}\\\" -Destination \\\"{}\\\" -Force' -Verb RunAs -WindowStyle Hidden",
        src_str, dst_str
    );
    let status = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_arg])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to rename/move as administrator")
    }
}

#[cfg(not(target_os = "windows"))]
fn run_as_admin_rename(src: &Path, dst: &Path) -> Result<()> {
    use std::process::Command;
    let status = Command::new("sudo").arg("mv").arg(src).arg(dst).status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to rename/move as administrator via sudo")
    }
}

/// Creates a new directory at the specified path.
pub fn create_directory(path: &Path, req_admin: bool) -> Result<()> {
    let res = fs::create_dir(path).context("Failed to create directory");
    if res.is_err() && req_admin {
        run_as_admin_mkdir(path)
    } else {
        res
    }
}

/// Renames or moves a file/directory synchronously.
/// On cross-device moves (different filesystems), falls back to copy + delete.
pub fn rename_or_move_sync(src: &Path, dst: &Path, req_admin: bool) -> Result<()> {
    let res = fs::rename(src, dst);
    match res {
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
            } else if req_admin {
                run_as_admin_rename(src, dst)
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
        delete_dir_recursive(src).context("Failed to remove source directory after cross-device move")
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

fn delete_dir_recursive(path: &Path) -> Result<()> {
    if path.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
        fs::remove_file(path).context("Failed to remove symlink")
    } else {
        let res = fs::remove_dir_all(path);
        #[cfg(not(target_os = "windows"))]
        {
            if res.is_err() {
                let status = std::process::Command::new("rm")
                    .arg("-rf")
                    .arg(path)
                    .status();
                if let Ok(s) = status {
                    if s.success() {
                        return Ok(());
                    }
                }
            }
        }
        res.context("Failed to delete directory recursively")
    }
}

/// Deletes a file or directory recursively.
pub fn delete_sync(path: &Path, delete_to_recycle_bin: bool, req_admin: bool) -> Result<()> {
    let res = if delete_to_recycle_bin {
        send_to_recycle_bin(path)
    } else {
        if path.is_dir() {
            delete_dir_recursive(path)
        } else {
            if let (Some(parent), Some(filename)) = (path.parent(), path.file_name()) {
                if let Some(filename_str) = filename.to_str() {
                    let _ = crate::fs::descriptions::remove_description(parent, filename_str);
                }
            }
            fs::remove_file(path).context("Failed to delete file")
        }
    };

    if res.is_err() && req_admin {
        run_as_admin_delete(path)
    } else {
        res
    }
}
