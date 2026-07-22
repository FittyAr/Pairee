use super::ProgressUpdate;
use super::helper::delete_recursive;
use crate::config::localization::t;
use crate::fs::ssh::SharedSshClient;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

/// Spawns a background task to copy or move multiple files between panels (supports remote panels).
pub fn spawn_copy_move_task(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
    src_conn: Option<SharedSshClient>,
    dst_conn: Option<SharedSshClient>,
    is_move: bool,
    _settings: crate::config::settings::Settings,
) -> mpsc::Receiver<ProgressUpdate> {
    if src_conn.is_none() && dst_conn.is_none() {
        panic!("spawn_copy_move_task must only be called for SSH transfers!");
    }

    let (tx, rx) = mpsc::channel(100);

    tokio::spawn(async move {
        // Helper to check if a path is a directory for a connection
        let is_dir_for_conn = |path: &Path, conn: &Option<SharedSshClient>| -> bool {
            if let Some(client) = conn {
                if let Ok(c) = client.0.lock() {
                    if let Ok(stat) = c.sftp.stat(path) {
                        return stat.is_dir();
                    }
                }
                false
            } else {
                path.is_dir()
            }
        };

        // If it is a remote-to-remote move on the same server, we can do a fast SFTP rename/move!
        if is_move {
            if let (Some(src_client), Some(dst_client)) = (&src_conn, &dst_conn) {
                if src_client.is_same_server(dst_client) {
                    let total_files = sources.len();
                    for (idx, src) in sources.iter().enumerate() {
                        let name = src
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        let dst = if crate::fs::transfer::worker::is_destination_parent_dir(
                            &sources,
                            &destination_dir,
                            |p| is_dir_for_conn(p, &dst_conn),
                        ) {
                            destination_dir.join(&name)
                        } else {
                            destination_dir.clone()
                        };

                        let _ = tx
                            .send(ProgressUpdate {
                                current_file: name.clone(),
                                files_copied: idx,
                                total_files,
                                bytes_copied: 0,
                                total_bytes: 0,
                                error: None,
                            })
                            .await;

                        if let Err(e) = src_client.rename_move(src, &dst) {
                            let err_msg =
                                t("error_remote_move_failed").replacen("{}", &e.to_string(), 1);
                            let _ = tx
                                .send(ProgressUpdate {
                                    current_file: name,
                                    files_copied: idx,
                                    total_files,
                                    bytes_copied: 0,
                                    total_bytes: 0,
                                    error: Some(err_msg),
                                })
                                .await;
                            return;
                        }
                    }

                    let _ = tx
                        .send(ProgressUpdate {
                            current_file: "Completed".to_string(),
                            files_copied: total_files,
                            total_files,
                            bytes_copied: 0,
                            total_bytes: 0,
                            error: None,
                        })
                        .await;
                    return;
                }
            }
        }

        // Otherwise, we perform copy/transfer logic (potentially followed by delete for move)
        let mut total_files = 0;
        let mut total_bytes = 0;
        let mut file_mappings = Vec::new();
        let mut dirs_to_create = Vec::new();

        let destination_dir_is_dir = crate::fs::transfer::worker::is_destination_parent_dir(
            &sources,
            &destination_dir,
            |p| is_dir_for_conn(p, &dst_conn),
        );

        for src in &sources {
            let is_dir = is_dir_for_conn(src, &src_conn);
            let name = src
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();

            if is_dir {
                let base_dst = if destination_dir_is_dir {
                    destination_dir.join(&name)
                } else {
                    destination_dir.clone()
                };
                dirs_to_create.push(base_dst.clone());

                // Scan remote or local directory
                if let Some(src_client) = &src_conn {
                    if let Ok(walked) = src_client.walk_dir(src) {
                        for (sub_src, sub_is_dir, sub_size) in walked {
                            if let Ok(rel) = sub_src.strip_prefix(src) {
                                let sub_dst = base_dst.join(rel);
                                if sub_is_dir {
                                    dirs_to_create.push(sub_dst);
                                } else {
                                    total_files += 1;
                                    total_bytes += sub_size;
                                    file_mappings.push((sub_src, sub_dst, sub_size));
                                }
                            }
                        }
                    }
                } else {
                    // Local directory walking
                    let mut dirs_to_visit = vec![src.clone()];
                    while let Some(dir) = dirs_to_visit.pop() {
                        if let Ok(entries) = std::fs::read_dir(&dir) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.is_dir() {
                                    dirs_to_visit.push(path.clone());
                                    if let Ok(rel) = path.strip_prefix(src) {
                                        dirs_to_create.push(base_dst.join(rel));
                                    }
                                } else {
                                    total_files += 1;
                                    let size = entry.metadata().ok().map(|m| m.len()).unwrap_or(0);
                                    total_bytes += size;
                                    if let Ok(rel) = path.strip_prefix(src) {
                                        let dest_path = base_dst.join(rel);
                                        file_mappings.push((path, dest_path, size));
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Single file mapping
                total_files += 1;
                let size = if let Some(src_client) = &src_conn {
                    if let Ok(c) = src_client.0.lock() {
                        c.sftp.stat(src).ok().and_then(|s| s.size).unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    src.metadata().ok().map(|m| m.len()).unwrap_or(0)
                };
                total_bytes += size;

                let dst_path = if destination_dir_is_dir {
                    destination_dir.join(&name)
                } else {
                    destination_dir.clone()
                };
                file_mappings.push((src.clone(), dst_path, size));
            }
        }

        // 2. Create the target folders
        for dir in &dirs_to_create {
            if let Some(dst_client) = &dst_conn {
                // Create parent directories as needed
                let mut current = PathBuf::new();
                for component in dir.components() {
                    current.push(component);
                    let _ = dst_client.create_dir(&current);
                }
            } else {
                let _ = std::fs::create_dir_all(dir);
            }
        }

        if file_mappings.is_empty() {
            // No files to copy (maybe only empty folders)
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

        // 3. Copy files block by block
        let mut files_copied = 0;
        let mut bytes_copied = 0;

        for (src, dst, _size) in file_mappings.clone() {
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

            let copy_res = async {
                // Open reader
                let mut reader: Box<dyn std::io::Read + Send> = if let Some(src_conn) = &src_conn {
                    let client = src_conn
                        .0
                        .lock()
                        .map_err(|_| anyhow::anyhow!(t("error_mutex_poisoned")))?;
                    let file = client.sftp.open(&src)?;
                    Box::new(file)
                } else {
                    let file = std::fs::File::open(&src)?;
                    Box::new(file)
                };

                // Open writer
                let mut writer: Box<dyn std::io::Write + Send> = if let Some(dst_conn) = &dst_conn {
                    let client = dst_conn
                        .0
                        .lock()
                        .map_err(|_| anyhow::anyhow!(t("error_mutex_poisoned")))?;
                    let file = client.sftp.create(&dst)?;
                    Box::new(file)
                } else {
                    if let Some(parent) = dst.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let file = std::fs::File::create(&dst)?;
                    Box::new(file)
                };

                // Perform chunked copy
                let mut buffer = vec![0; 64 * 1024];
                let throttle = std::time::Duration::from_millis(100);
                let mut last_sent = std::time::Instant::now();

                loop {
                    let bytes_read = reader.read(&mut buffer)?;
                    if bytes_read == 0 {
                        break;
                    }
                    writer.write_all(&buffer[..bytes_read])?;
                    bytes_copied += bytes_read as u64;

                    tokio::task::yield_now().await;

                    if last_sent.elapsed() >= throttle {
                        last_sent = std::time::Instant::now();
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
                    }
                }
                Ok::<(), anyhow::Error>(())
            }
            .await;

            if let Err(e) = copy_res {
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

            files_copied += 1;
        }

        // 4. Delete sources if it is a move operation
        if is_move {
            for src in &sources {
                if let Some(src_client) = &src_conn {
                    if let Err(e) = src_client.delete_recursive(src) {
                        let err_msg = t("error_remote_source_delete_failed")
                            .replacen("{}", &src.to_string_lossy(), 1)
                            .replacen("{}", &e.to_string(), 1);
                        let _ = tx
                            .send(ProgressUpdate {
                                current_file: "Completed".to_string(),
                                files_copied,
                                total_files,
                                bytes_copied,
                                total_bytes,
                                error: Some(err_msg),
                            })
                            .await;
                        return;
                    }
                } else {
                    if let Err(e) = delete_recursive(src) {
                        let err_msg = t("error_delete_source_failed")
                            .replacen("{}", &src.to_string_lossy(), 1)
                            .replacen("{}", &e.to_string(), 1);
                        let _ = tx
                            .send(ProgressUpdate {
                                current_file: "Completed".to_string(),
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

        // 5. Done!
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
