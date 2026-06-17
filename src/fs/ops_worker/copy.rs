use super::ProgressUpdate;
use super::helper::{copy_file_buffered, copy_symlink, run_as_admin_copy};
use crate::config::localization::t;
use std::fs;
use std::path::PathBuf;
use tokio::sync::mpsc;

pub fn spawn_copy_task(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
    settings: crate::config::settings::Settings,
) -> mpsc::Receiver<ProgressUpdate> {
    let (tx, rx) = mpsc::channel(100);

    tokio::spawn(async move {
        let mut total_files = 0;
        let mut total_bytes = 0;
        let mut file_mappings = Vec::new();
        let mut dirs_to_create = Vec::new();

        // 1. Gather all directories to create and files to copy
        for src in &sources {
            let is_sym = src.is_symlink();
            if src.is_dir() && (!is_sym || settings.scan_symbolic_links) {
                if let Some(folder_name) = src.file_name() {
                    let base_dst = if destination_dir.is_dir() {
                        destination_dir.join(folder_name)
                    } else {
                        destination_dir.clone()
                    };
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
                let dst_path = if destination_dir.is_dir() {
                    if let Some(file_name) = src.file_name() {
                        destination_dir.join(file_name)
                    } else {
                        continue;
                    }
                } else {
                    destination_dir.clone()
                };
                file_mappings.push((src.clone(), dst_path, is_sym));
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
                            error: Some(format!("{} {:?}: {}", t("error_mkdir_failed"), dir, e)),
                        })
                        .await;
                    return;
                }
            }
        }

        // 3. Copy files block by block
        let mut files_copied = 0;
        let mut bytes_copied = 0;

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
                        let err_msg = t("error_copying_to")
                            .replacen("{}", &src.to_string_lossy(), 1)
                            .replacen("{}", &dst.to_string_lossy(), 1)
                            .replacen("{}", &e.to_string(), 1);
                        let _ = tx
                            .send(ProgressUpdate {
                                current_file: file_name,
                                files_copied,
                                total_files,
                                bytes_copied,
                                total_bytes,
                                error: Some(err_msg),
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
                        let err_msg = t("error_copying_to")
                            .replacen("{}", &src.to_string_lossy(), 1)
                            .replacen("{}", &dst.to_string_lossy(), 1)
                            .replacen("{}", &e.to_string(), 1);
                        let _ = tx
                            .send(ProgressUpdate {
                                current_file: file_name,
                                files_copied,
                                total_files,
                                bytes_copied,
                                total_bytes,
                                error: Some(err_msg),
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
                        let err_msg = t("error_copying_to")
                            .replacen("{}", &src.to_string_lossy(), 1)
                            .replacen("{}", &dst.to_string_lossy(), 1)
                            .replacen("{}", &e.to_string(), 1);
                        let _ = tx
                            .send(ProgressUpdate {
                                current_file: file_name,
                                files_copied,
                                total_files,
                                bytes_copied,
                                total_bytes,
                                error: Some(err_msg),
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
