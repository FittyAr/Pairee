use std::fs;
use std::path::PathBuf;
use tokio::sync::mpsc;
use super::ProgressUpdate;
use crate::config::localization::t;
use super::helper::{
    run_as_admin_copy, delete_recursive, copy_dir_recursive_async, copy_symlink, copy_file_buffered,
};

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
                            } else if let Err(err_admin) = res {
                                let err_msg = t("error_moving_failed_for")
                                    .replacen("{}", &src.to_string_lossy(), 1)
                                    .replacen("{}", &err_admin.to_string(), 1);
                                let _ = tx
                                    .send(ProgressUpdate {
                                        current_file: file_name,
                                        files_copied,
                                        total_files,
                                        bytes_copied: 0,
                                        total_bytes: 0,
                                        error: Some(err_msg),
                                    })
                                    .await;
                                return;
                            }
                        } else {
                            let e_msg = rename_res.err().unwrap().to_string();
                            let err_msg = t("error_moving_failed_for")
                                .replacen("{}", &src.to_string_lossy(), 1)
                                .replacen("{}", &e_msg, 1);
                            let _ = tx
                                .send(ProgressUpdate {
                                    current_file: file_name,
                                    files_copied,
                                    total_files,
                                    bytes_copied: 0,
                                    total_bytes: 0,
                                    error: Some(err_msg),
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
                        let err_msg = t("error_delete_source_failed")
                            .replacen("{}", &src.to_string_lossy(), 1)
                            .replacen("{}", &e.to_string(), 1);
                        let _ = tx
                            .send(ProgressUpdate {
                                current_file: file_name,
                                files_copied,
                                total_files,
                                bytes_copied,
                                total_bytes: item_total_bytes,
                                error: Some(err_msg),
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
                    let err_msg = t("error_moving_failed_for")
                        .replacen("{}", &src.to_string_lossy(), 1)
                        .replacen("{}", &e.to_string(), 1);
                    let _ = tx
                        .send(ProgressUpdate {
                            current_file: file_name,
                            files_copied,
                            total_files,
                            bytes_copied,
                            total_bytes: item_total_bytes,
                            error: Some(err_msg),
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
