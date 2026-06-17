use anyhow::Result;
use std::fs;
use std::path::Path;
use tokio::sync::mpsc;
use super::ProgressUpdate;
use crate::config::localization::t;

pub(crate) fn copy_symlink(src: &Path, dst: &Path) -> Result<()> {
    let target = fs::read_link(src)?;
    #[cfg(target_os = "windows")]
    {
        let resolved_target = if target.is_relative() {
            src.parent()
                .map(|p| p.join(&target))
                .unwrap_or_else(|| target.clone())
        } else {
            target.clone()
        };
        if resolved_target.is_dir() {
            std::os::windows::fs::symlink_dir(&target, dst)?;
        } else {
            std::os::windows::fs::symlink_file(&target, dst)?;
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::os::unix::fs::symlink(&target, dst)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub(crate) fn run_as_admin_copy(src: &Path, dst: &Path) -> Result<()> {
    use std::process::Command;
    let src_str = src.to_string_lossy().replace('"', "\\\"");
    let dst_str = dst.to_string_lossy().replace('"', "\\\"");
    let ps_arg = format!(
        "Start-Process powershell -ArgumentList '-NoProfile -Command Copy-Item -Path \\\"{}\\\" -Destination \\\"{}\\\" -Force' -Verb RunAs -WindowStyle Hidden -Wait",
        src_str, dst_str
    );
    let status = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_arg])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!(t("error_failed_copy_admin"))
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn run_as_admin_copy(src: &Path, dst: &Path) -> Result<()> {
    use std::process::Command;
    let status = Command::new("sudo")
        .arg("cp")
        .arg("-p")
        .arg(src)
        .arg(dst)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!(t("error_failed_copy_sudo"))
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn copy_file_buffered(
    src: &Path,
    dst: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
    global_bytes_copied: &mut u64,
    file_name: &str,
    files_copied: usize,
    total_files: usize,
    total_bytes: u64,
    copy_files_opened_for_writing: bool,
) -> Result<()> {
    use std::io::{Read, Write};
    use std::time::{Duration, Instant};
    let mut src_file = if copy_files_opened_for_writing {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .read(true)
                .share_mode(7) // FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE
                .open(src)?
        }
        #[cfg(not(target_os = "windows"))]
        {
            fs::File::open(src)?
        }
    } else {
        fs::File::open(src)?
    };
    let mut dst_file = fs::File::create(dst)?;

    let mut buffer = vec![0; 64 * 1024]; // 64 KB buffer size
    let throttle = Duration::from_millis(100);
    let mut last_sent = Instant::now();
    loop {
        let bytes_read = src_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        dst_file.write_all(&buffer[..bytes_read])?;
        *global_bytes_copied += bytes_read as u64;

        // Yield to the async runtime so the UI loop keeps running
        tokio::task::yield_now().await;

        // Throttle progress updates to ~10 per second
        if last_sent.elapsed() >= throttle {
            last_sent = Instant::now();
            let _ = tx
                .send(ProgressUpdate {
                    current_file: file_name.to_string(),
                    files_copied,
                    total_files,
                    bytes_copied: *global_bytes_copied,
                    total_bytes,
                    error: None,
                })
                .await;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn copy_dir_recursive_async(
    src: &Path,
    dst: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
    files_copied: &mut usize,
    bytes_copied: &mut u64,
    total_files: usize,
    total_bytes: u64,
    copy_files_opened_for_writing: bool,
) -> Result<()> {
    fs::create_dir_all(dst)
        .map_err(|e| anyhow::anyhow!(t("error_failed_create_dir").replacen("{}", &dst.to_string_lossy(), 1).replacen("{}", &e.to_string(), 1)))?;
    for entry in
        fs::read_dir(src).map_err(|e| anyhow::anyhow!(t("error_failed_read_dir").replacen("{}", &src.to_string_lossy(), 1).replacen("{}", &e.to_string(), 1)))?
    {
        let entry =
            entry.map_err(|e| anyhow::anyhow!(t("error_failed_read_dir_entry").replacen("{}", &e.to_string(), 1)))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_name = entry
            .file_name()
            .to_string_lossy()
            .into_owned();
        if src_path.is_dir() {
            Box::pin(copy_dir_recursive_async(
                &src_path,
                &dst_path,
                tx,
                files_copied,
                bytes_copied,
                total_files,
                total_bytes,
                copy_files_opened_for_writing,
            ))
            .await?;
        } else {
            copy_file_buffered(
                &src_path,
                &dst_path,
                tx,
                bytes_copied,
                &file_name,
                *files_copied,
                total_files,
                total_bytes,
                copy_files_opened_for_writing,
            )
            .await?;
            *files_copied += 1;
        }
    }
    Ok(())
}

pub(crate) fn delete_recursive(path: &Path) -> Result<()> {
    if path
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        fs::remove_file(path)
            .map_err(|e| anyhow::anyhow!(t("error_failed_remove_symlink").replacen("{}", &path.to_string_lossy(), 1).replacen("{}", &e.to_string(), 1)))
    } else if path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|e| anyhow::anyhow!(t("error_failed_delete_dir").replacen("{}", &path.to_string_lossy(), 1).replacen("{}", &e.to_string(), 1)))
    } else {
        fs::remove_file(path)
            .map_err(|e| anyhow::anyhow!(t("error_failed_delete_file").replacen("{}", &path.to_string_lossy(), 1).replacen("{}", &e.to_string(), 1)))
    }
}
