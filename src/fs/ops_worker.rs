use crate::fs::archive::{compress_zip, extract_archive};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    /// Name of the file currently being copied/moved
    pub current_file: String,
    /// Number of files fully copied so far
    pub files_copied: usize,
    /// Total number of files to copy
    pub total_files: usize,
    /// Total number of bytes copied so far across all files
    pub bytes_copied: u64,
    /// Total bytes to copy across all files
    pub total_bytes: u64,
    /// Detailed error message if the task fails
    pub error: Option<String>,
}

fn copy_symlink(src: &Path, dst: &Path) -> Result<()> {
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
fn run_as_admin_copy(src: &Path, dst: &Path) -> Result<()> {
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
        anyhow::bail!("Failed to copy as administrator")
    }
}

#[cfg(not(target_os = "windows"))]
fn run_as_admin_copy(src: &Path, dst: &Path) -> Result<()> {
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
        anyhow::bail!("Failed to copy as administrator via sudo")
    }
}

/// Spawns a background task to copy multiple source files/directories to a destination directory.
/// Returns a channel receiver for real-time progress updates.
pub fn spawn_copy_task(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
    settings: crate::config::settings::Settings,
) -> mpsc::Receiver<ProgressUpdate> {
    let (tx, rx) = mpsc::channel(100);

    tokio::spawn(async move {
        let mut total_files = 0;
        let mut total_bytes = 0;
        let mut file_mappings = Vec::new(); // (src, dst, is_symlink)
        let mut dirs_to_create = Vec::new();

        // 1. Gather all directories to create and files to copy
        for src in &sources {
            let is_sym = src.is_symlink();
            if src.is_dir() && (!is_sym || settings.scan_symbolic_links) {
                if let Some(folder_name) = src.file_name() {
                    let base_dst = destination_dir.join(folder_name);
                    dirs_to_create.push(base_dst.clone());

                    let mut dirs_to_visit = vec![src.clone()];
                    while let Some(dir) = dirs_to_visit.pop() {
                        if let Ok(entries) = fs::read_dir(&dir) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                let entry_is_sym = path.is_symlink();
                                if path.is_dir() && (!entry_is_sym || settings.scan_symbolic_links)
                                {
                                    dirs_to_visit.push(path.clone());
                                    if let Ok(rel) = path.strip_prefix(src) {
                                        let dst_dir = base_dst.join(rel);
                                        dirs_to_create.push(dst_dir);
                                    }
                                } else {
                                    total_files += 1;
                                    if !entry_is_sym {
                                        if let Ok(meta) = entry.metadata() {
                                            total_bytes += meta.len();
                                        }
                                    }
                                    if let Ok(rel) = path.strip_prefix(src) {
                                        let dst_path = base_dst.join(rel);
                                        file_mappings.push((path, dst_path, entry_is_sym));
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                total_files += 1;
                if !is_sym {
                    if let Ok(meta) = src.metadata() {
                        total_bytes += meta.len();
                    }
                }
                if let Some(file_name) = src.file_name() {
                    let dst_path = destination_dir.join(file_name);
                    file_mappings.push((src.clone(), dst_path, is_sym));
                }
            }
        }

        // 2. Create the target folder structures
        for dir in dirs_to_create {
            let res = fs::create_dir_all(&dir);
            if res.is_err() {
                let admin_res = if settings.req_admin_modification {
                    #[cfg(target_os = "windows")]
                    {
                        use std::process::Command;
                        let dir_str = dir.to_string_lossy().replace('"', "\\\"");
                        let ps_arg = format!(
                            "Start-Process powershell -ArgumentList '-NoProfile -Command New-Item -ItemType Directory -Path \\\"{}\\\" -Force' -Verb RunAs -WindowStyle Hidden -Wait",
                            dir_str
                        );
                        Command::new("powershell")
                            .args(&["-NoProfile", "-Command", &ps_arg])
                            .status()
                            .map(|s| s.success())
                            .unwrap_or(false)
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        use std::process::Command;
                        Command::new("sudo")
                            .arg("mkdir")
                            .arg("-p")
                            .arg(&dir)
                            .status()
                            .map(|s| s.success())
                            .unwrap_or(false)
                    }
                } else {
                    false
                };

                if !admin_res {
                    let e = res.err().unwrap();
                    let _ = tx
                        .send(ProgressUpdate {
                            current_file: dir.to_string_lossy().into_owned(),
                            files_copied: 0,
                            total_files,
                            bytes_copied: 0,
                            total_bytes,
                            error: Some(format!("Failed to create folder {:?}: {}", dir, e)),
                        })
                        .await;
                    return;
                }
            }
        }

        // 3. Copy files block by block
        let mut files_copied = 0;
        let mut bytes_copied = 0;

        // In case there were only empty folders, trigger a finish
        if file_mappings.is_empty() {
            let _ = tx
                .send(ProgressUpdate {
                    current_file: "Completed".to_string(),
                    files_copied: total_files,
                    total_files,
                    bytes_copied: total_bytes,
                    total_bytes,
                    error: None,
                })
                .await;
            return;
        }

        for (src, dst, is_sym) in file_mappings {
            let file_name = src
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();

            // Send starting file notification
            let _ = tx
                .send(ProgressUpdate {
                    current_file: file_name.clone(),
                    files_copied,
                    total_files,
                    bytes_copied,
                    total_bytes,
                    error: None,
                })
                .await;

            if let Some(parent) = dst.parent() {
                let _ = fs::create_dir_all(parent);
            }

            if is_sym {
                let mut res = copy_symlink(&src, &dst);
                if res.is_err() && settings.req_admin_modification {
                    res = run_as_admin_copy(&src, &dst);
                }
                match res {
                    Ok(_) => {
                        files_copied += 1;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(ProgressUpdate {
                                current_file: file_name,
                                files_copied,
                                total_files,
                                bytes_copied,
                                total_bytes,
                                error: Some(format!("Error copying symlink {:?}: {}", src, e)),
                            })
                            .await;
                        return;
                    }
                }
            } else if settings.use_system_copy_routine {
                let mut res: anyhow::Result<()> =
                    std::fs::copy(&src, &dst).map(|_| ()).map_err(|e| e.into());
                if res.is_err() && settings.req_admin_modification {
                    res = run_as_admin_copy(&src, &dst);
                }
                match res {
                    Ok(_) => {
                        if let Ok(meta) = src.metadata() {
                            bytes_copied += meta.len();
                        }
                        files_copied += 1;
                        let _ = tx
                            .send(ProgressUpdate {
                                current_file: file_name,
                                files_copied,
                                total_files,
                                bytes_copied,
                                total_bytes,
                                error: None,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(ProgressUpdate {
                                current_file: file_name,
                                files_copied,
                                total_files,
                                bytes_copied,
                                total_bytes,
                                error: Some(format!("Error copying file {:?}: {}", src, e)),
                            })
                            .await;
                        return;
                    }
                }
            } else {
                let mut res = copy_file_buffered(
                    &src,
                    &dst,
                    &tx,
                    &mut bytes_copied,
                    &file_name,
                    files_copied,
                    total_files,
                    total_bytes,
                    settings.copy_files_opened_for_writing,
                )
                .await;
                if res.is_err() && settings.req_admin_modification {
                    res = run_as_admin_copy(&src, &dst);
                    if res.is_ok() {
                        if let Ok(meta) = src.metadata() {
                            bytes_copied += meta.len();
                        }
                    }
                }
                match res {
                    Ok(_) => {
                        files_copied += 1;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(ProgressUpdate {
                                current_file: file_name,
                                files_copied,
                                total_files,
                                bytes_copied,
                                total_bytes,
                                error: Some(format!("Error copying file {:?}: {}", src, e)),
                            })
                            .await;
                        return;
                    }
                }
            }
        }

        // 4. Send final completion update
        let _ = tx
            .send(ProgressUpdate {
                current_file: "Completed".to_string(),
                files_copied,
                total_files,
                bytes_copied,
                total_bytes,
                error: None,
            })
            .await;
    });

    rx
}

/// Copies a single file in chunks to allow cancellation or smooth progress updates.
/// Progress updates are throttled to every 100 ms to avoid flooding the channel.
#[allow(clippy::too_many_arguments)]
async fn copy_file_buffered(
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

/// Recursively copies a directory tree, reporting progress via the channel.
async fn copy_dir_recursive_async(
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
        .map_err(|e| anyhow::anyhow!("Failed to create directory {:?}: {}", dst, e))?;
    for entry in
        fs::read_dir(src).map_err(|e| anyhow::anyhow!("Failed to read dir {:?}: {}", src, e))?
    {
        let entry =
            entry.map_err(|e| anyhow::anyhow!("Failed to read dir entry: {}", e))?;
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

/// Recursively deletes a directory or file (helper for async move fallback).
fn delete_recursive(path: &Path) -> Result<()> {
    if path
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        fs::remove_file(path)
            .map_err(|e| anyhow::anyhow!("Failed to remove symlink {:?}: {}", path, e))
    } else if path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|e| anyhow::anyhow!("Failed to delete dir {:?}: {}", path, e))
    } else {
        fs::remove_file(path)
            .map_err(|e| anyhow::anyhow!("Failed to delete file {:?}: {}", path, e))
    }
}

/// Spawns a background task that moves multiple source files/directories to a destination directory.
/// It first tries a fast atomic rename; on cross-device failures it falls back to copy + delete.
/// Returns a channel receiver for real-time progress updates.
pub fn spawn_move_task(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
    settings: crate::config::settings::Settings,
) -> mpsc::Receiver<ProgressUpdate> {
    let (tx, rx) = mpsc::channel(100);

    tokio::spawn(async move {
        let total_files = sources.len();
        let mut files_copied = 0usize;

        for src in &sources {
            let file_name = src
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();

            // Determine destination path
            let dst = if sources.len() == 1 && !destination_dir.is_dir() {
                destination_dir.clone()
            } else {
                destination_dir.join(&file_name)
            };

            // Send starting notification
            let _ = tx
                .send(ProgressUpdate {
                    current_file: file_name.clone(),
                    files_copied,
                    total_files,
                    bytes_copied: 0,
                    total_bytes: 0,
                    error: None,
                })
                .await;

            // Try fast atomic rename first
            let rename_res = fs::rename(src, &dst);
            match rename_res {
                Ok(()) => {
                    files_copied += 1;
                    let _ = tx
                        .send(ProgressUpdate {
                            current_file: file_name.clone(),
                            files_copied,
                            total_files,
                            bytes_copied: 0,
                            total_bytes: 0,
                            error: None,
                        })
                        .await;
                    continue;
                }
                Err(ref e) => {
                    let is_cross_device = e
                        .raw_os_error()
                        .map(|code| code == 18 || code == 17)
                        .unwrap_or(false);
                    let is_cross = is_cross_device
                        || e.kind() == std::io::ErrorKind::CrossesDevices;

                    if !is_cross {
                        // Permission or other error — try admin if configured
                        if settings.req_admin_modification {
                            let res = run_as_admin_copy(src, &dst);
                            if res.is_ok() {
                                let _ = delete_recursive(src);
                                files_copied += 1;
                                let _ = tx
                                    .send(ProgressUpdate {
                                        current_file: file_name,
                                        files_copied,
                                        total_files,
                                        bytes_copied: 0,
                                        total_bytes: 0,
                                        error: None,
                                    })
                                    .await;
                                continue;
                            } else if let Err(e) = res {
                                let _ = tx
                                    .send(ProgressUpdate {
                                        current_file: file_name,
                                        files_copied,
                                        total_files,
                                        bytes_copied: 0,
                                        total_bytes: 0,
                                        error: Some(format!("Error moving {:?}: {}", src, e)),
                                    })
                                    .await;
                                return;
                            }
                        } else {
                            let e_msg = rename_res.err().unwrap().to_string();
                            let _ = tx
                                .send(ProgressUpdate {
                                    current_file: file_name,
                                    files_copied,
                                    total_files,
                                    bytes_copied: 0,
                                    total_bytes: 0,
                                    error: Some(format!("Error moving {:?}: {}", src, e_msg)),
                                })
                                .await;
                            return;
                        }
                    }
                }
            }

            // Cross-device fallback: compute total bytes for this item
            let mut item_total_bytes = 0u64;
            if src.is_dir() {
                let mut dirs_to_visit = vec![src.clone()];
                while let Some(dir) = dirs_to_visit.pop() {
                    if let Ok(entries) = fs::read_dir(&dir) {
                        for entry in entries.flatten() {
                            let p = entry.path();
                            if p.is_dir() {
                                dirs_to_visit.push(p);
                            } else if !p.is_symlink() {
                                if let Ok(meta) = entry.metadata() {
                                    item_total_bytes += meta.len();
                                }
                            }
                        }
                    }
                }
            } else if !src.is_symlink() {
                if let Ok(meta) = src.metadata() {
                    item_total_bytes = meta.len();
                }
            }

            let mut bytes_copied = 0u64;

            // Cross-device copy phase
            let copy_res = if src.is_dir() {
                let is_sym = src.is_symlink();
                if is_sym && !settings.scan_symbolic_links {
                    Ok(())
                } else {
                    copy_dir_recursive_async(
                        src,
                        &dst,
                        &tx,
                        &mut files_copied,
                        &mut bytes_copied,
                        total_files,
                        item_total_bytes,
                        settings.copy_files_opened_for_writing,
                    )
                    .await
                }
            } else {
                let is_sym = src.is_symlink();
                if is_sym {
                    copy_symlink(src, &dst).map_err(|e| e)
                } else {
                    copy_file_buffered(
                        src,
                        &dst,
                        &tx,
                        &mut bytes_copied,
                        &file_name,
                        files_copied,
                        total_files,
                        item_total_bytes,
                        settings.copy_files_opened_for_writing,
                    )
                    .await
                }
            };

            match copy_res {
                Ok(()) => {
                    // Delete source after successful copy
                    if let Err(e) = delete_recursive(src) {
                        let _ = tx
                            .send(ProgressUpdate {
                                current_file: file_name,
                                files_copied,
                                total_files,
                                bytes_copied,
                                total_bytes: item_total_bytes,
                                error: Some(format!(
                                    "Copied but failed to remove source {:?}: {}",
                                    src, e
                                )),
                            })
                            .await;
                        return;
                    }
                    files_copied += 1;
                    let _ = tx
                        .send(ProgressUpdate {
                            current_file: file_name,
                            files_copied,
                            total_files,
                            bytes_copied,
                            total_bytes: item_total_bytes,
                            error: None,
                        })
                        .await;
                }
                Err(e) => {
                    let _ = tx
                        .send(ProgressUpdate {
                            current_file: file_name,
                            files_copied,
                            total_files,
                            bytes_copied,
                            total_bytes: item_total_bytes,
                            error: Some(format!("Error moving {:?}: {}", src, e)),
                        })
                        .await;
                    return;
                }
            }
        }

        // Final completion update
        let _ = tx
            .send(ProgressUpdate {
                current_file: "Completed".to_string(),
                files_copied,
                total_files,
                bytes_copied: 0,
                total_bytes: 0,
                error: None,
            })
            .await;
    });

    rx
}


pub fn spawn_extract_task(
    archive_path: PathBuf,
    destination_dir: PathBuf,
) -> mpsc::Receiver<ProgressUpdate> {
    let (tx, rx) = mpsc::channel(100);
    tokio::task::spawn_blocking(move || {
        if let Err(e) = extract_archive(&archive_path, &destination_dir, &tx) {
            let _ = tx.blocking_send(ProgressUpdate {
                current_file: archive_path.to_string_lossy().into_owned(),
                files_copied: 0,
                total_files: 0,
                bytes_copied: 0,
                total_bytes: 0,
                error: Some(format!("Extraction failed: {}", e)),
            });
        } else {
            let _ = tx.blocking_send(ProgressUpdate {
                current_file: "Completed".to_string(),
                files_copied: 1,
                total_files: 1,
                bytes_copied: 0,
                total_bytes: 0,
                error: None,
            });
        }
    });
    rx
}

pub fn spawn_compress_task(
    sources: Vec<PathBuf>,
    dest_archive: PathBuf,
) -> mpsc::Receiver<ProgressUpdate> {
    let (tx, rx) = mpsc::channel(100);
    tokio::task::spawn_blocking(move || {
        if let Err(e) = compress_zip(sources, &dest_archive, &tx) {
            let _ = tx.blocking_send(ProgressUpdate {
                current_file: dest_archive.to_string_lossy().into_owned(),
                files_copied: 0,
                total_files: 0,
                bytes_copied: 0,
                total_bytes: 0,
                error: Some(format!("Compression failed: {}", e)),
            });
        } else {
            let _ = tx.blocking_send(ProgressUpdate {
                current_file: "Completed".to_string(),
                files_copied: 1,
                total_files: 1,
                bytes_copied: 0,
                total_bytes: 0,
                error: None,
            });
        }
    });
    rx
}

/// Spawns a background task that securely wipes each file in `targets`.
/// Uses the same progress channel pattern as `spawn_copy_task`.
pub fn spawn_wipe_task(targets: Vec<PathBuf>) -> mpsc::Receiver<ProgressUpdate> {
    let (tx, rx) = mpsc::channel(64);
    let total = targets.len();

    tokio::task::spawn_blocking(move || {
        for (idx, path) in targets.iter().enumerate() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());

            let _ = tx.blocking_send(ProgressUpdate {
                current_file: name.clone(),
                files_copied: idx,
                total_files: total,
                bytes_copied: 0,
                total_bytes: 0,
                error: None,
            });

            if let Err(e) = crate::fs::wipe::wipe_file(path) {
                let _ = tx.blocking_send(ProgressUpdate {
                    current_file: "Completed".to_string(),
                    files_copied: idx,
                    total_files: total,
                    bytes_copied: 0,
                    total_bytes: 0,
                    error: Some(format!("Wipe failed for {:?}: {}", path, e)),
                });
                return;
            }
        }

        let _ = tx.blocking_send(ProgressUpdate {
            current_file: "Completed".to_string(),
            files_copied: total,
            total_files: total,
            bytes_copied: 0,
            total_bytes: 0,
            error: None,
        });
    });

    rx
}
