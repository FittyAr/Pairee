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

fn run_as_admin_delete(path: &Path) -> Result<()> {
    crate::fs::run_in_elevated_helper(vec![
        crate::fs::FsOperation::Delete { path: path.to_path_buf() }
    ])
}

pub(crate) fn make_writable(path: &Path) -> std::io::Result<()> {
    let metadata = path.symlink_metadata()?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    let mut perms = metadata.permissions();
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = perms.mode();
        let is_dir = metadata.is_dir();
        let new_mode = if is_dir { mode | 0o700 } else { mode | 0o600 };
        perms.set_mode(new_mode);
    }
    #[cfg(target_os = "windows")]
    {
        perms.set_readonly(false);
    }
    fs::set_permissions(path, perms)
}

fn ensure_writable_recursive(path: &Path) -> std::io::Result<()> {
    let metadata = path.symlink_metadata()?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }

    if metadata.is_dir() {
        let _ = make_writable(path);
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let _ = ensure_writable_recursive(&entry.path());
            }
        }
        let _ = make_writable(path);
    } else {
        let _ = make_writable(path);
    }
    Ok(())
}

pub(crate) fn delete_dir_recursive(path: &Path) -> Result<()> {
    if path
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        fs::remove_file(path).context("Failed to remove symlink")
    } else {
        let mut res = fs::remove_dir_all(path);
        if res.is_err() {
            let _ = ensure_writable_recursive(path);
            res = fs::remove_dir_all(path);
        }
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
            let file_res = fs::remove_file(path);
            if file_res.is_err() {
                let _ = make_writable(path);
                fs::remove_file(path).context("Failed to delete file")
            } else {
                file_res.context("Failed to delete file")
            }
        }
    };

    if res.is_err() && req_admin {
        run_as_admin_delete(path)
    } else {
        res
    }
}
