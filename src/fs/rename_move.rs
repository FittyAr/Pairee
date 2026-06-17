use super::delete::delete_dir_recursive;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

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
    use crossterm::cursor::Show;
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };
    use std::process::{Command, Stdio};

    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen, Show);

    println!(
        "\nRequesting administrator privileges to move/rename:\n  From: {}\n  To:   {}",
        src.display(),
        dst.display()
    );

    let status = Command::new("sudo")
        .arg("mv")
        .arg(src)
        .arg(dst)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    let _ = enable_raw_mode();
    let _ = execute!(std::io::stdout(), EnterAlternateScreen);

    match status {
        Ok(s) if s.success() => Ok(()),
        _ => anyhow::bail!("Failed to rename/move as administrator via sudo"),
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

/// Copies a file or directory tree to `dst`, then removes the source.
/// Used as a fallback when rename fails across filesystem boundaries.
pub fn move_with_fallback(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        copy_dir_recursive(src, dst)?;
        delete_dir_recursive(src)
            .context("Failed to remove source directory after cross-device move")
    } else {
        fs::copy(src, dst).context("Failed to copy file for cross-device move")?;
        fs::remove_file(src).context("Failed to remove source file after cross-device move")
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
