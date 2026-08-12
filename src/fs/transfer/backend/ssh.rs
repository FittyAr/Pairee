//! SSH/SFTP transfer backend.
//!
//! Ports the former `ops_worker::copy_move` / delete paths onto
//! [`TransferEvent`] so the UI uses a single progress model.

use super::super::events::TransferEvent;
use super::super::job::{FailedFile, FileTransferResult, TransferOperation, TransferResults};
use super::super::worker::is_destination_parent_dir;
use super::BackendControl;
use crate::config::localization::t;
use crate::fs::delete_util::delete_recursive;
use crate::fs::ssh::SharedSshClient;
use crate::fs::transfer::job::SshEndpoints;
use anyhow::anyhow;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub async fn run_ssh_job(
    operation: TransferOperation,
    sources: Vec<PathBuf>,
    destination: PathBuf,
    ssh: SshEndpoints,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    match operation {
        TransferOperation::Delete => run_ssh_delete(sources, ssh, control).await,
        TransferOperation::Copy => {
            run_ssh_copy_move(sources, destination, ssh, false, control).await
        }
        TransferOperation::Move => {
            run_ssh_copy_move(sources, destination, ssh, true, control).await
        }
        TransferOperation::Wipe
        | TransferOperation::Compress
        | TransferOperation::Extract
        | TransferOperation::ApplyCommand => {
            Err(anyhow!("{} is not available over SSH", operation.label()))
        }
    }
}

async fn run_ssh_delete(
    sources: Vec<PathBuf>,
    ssh: SshEndpoints,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    let client = ssh
        .src
        .or(ssh.dst)
        .ok_or_else(|| anyhow!("SSH delete requires a connection"))?;

    let total = sources.len();
    let _ = control.event_tx.send(TransferEvent::ScanComplete {
        job_id: control.job_id,
        total_files: total,
        total_bytes: 0,
    });

    let mut results = TransferResults::default();

    for (idx, path) in sources.iter().enumerate() {
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }
        control.wait_if_paused();
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }

        let start = Instant::now();
        let _ = control.event_tx.send(TransferEvent::FileStarted {
            job_id: control.job_id,
            file: path.clone(),
            index: idx,
        });

        match client.delete_recursive(path) {
            Ok(()) => {
                let result = FileTransferResult {
                    src: path.clone(),
                    dst: PathBuf::new(),
                    size: 0,
                    src_hash: None,
                    dst_hash: None,
                    verified: true,
                    duration: start.elapsed(),
                };
                results.completed_files.push(result.clone());
                let _ = control.event_tx.send(TransferEvent::FileCompleted {
                    job_id: control.job_id,
                    result,
                });
            }
            Err(e) => {
                let failed = FailedFile {
                    src: path.clone(),
                    dst: PathBuf::new(),
                    error: e.to_string(),
                    retries: 0,
                };
                results.failed_files.push(failed.clone());
                let _ = control.event_tx.send(TransferEvent::FileFailed {
                    job_id: control.job_id,
                    error: failed,
                });
                return Err(anyhow!(e.to_string()));
            }
        }
    }

    let _ = control.event_tx.send(TransferEvent::JobCompleted {
        job_id: control.job_id,
        results: results.clone(),
    });
    Ok(results)
}

async fn run_ssh_copy_move(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
    ssh: SshEndpoints,
    is_move: bool,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    let src_conn = ssh.src;
    let dst_conn = ssh.dst;

    if src_conn.is_none() && dst_conn.is_none() {
        return Err(anyhow!("SSH transfer requires at least one connection"));
    }

    // Same-server fast move via SFTP rename
    if is_move
        && let (Some(src_client), Some(dst_client)) = (&src_conn, &dst_conn)
        && src_client.is_same_server(dst_client)
    {
        return fast_remote_rename(sources, destination_dir, src_client, &dst_conn, control).await;
    }

    let is_dir_for_conn = |path: &Path, conn: &Option<SharedSshClient>| -> bool {
        if let Some(client) = conn {
            if let Ok(c) = client.0.lock()
                && let Ok(stat) = c.sftp.stat(path)
            {
                return stat.is_dir();
            }
            false
        } else {
            path.is_dir()
        }
    };

    let mut total_files = 0usize;
    let mut total_bytes = 0u64;
    let mut file_mappings = Vec::new();
    let mut dirs_to_create = Vec::new();

    let destination_dir_is_dir = is_destination_parent_dir(&sources, &destination_dir, |p| {
        is_dir_for_conn(p, &dst_conn)
    });

    for src in &sources {
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }
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

        let _ = control.event_tx.send(TransferEvent::ScanProgress {
            job_id: control.job_id,
            files_found: total_files,
        });
    }

    let _ = control.event_tx.send(TransferEvent::ScanComplete {
        job_id: control.job_id,
        total_files,
        total_bytes,
    });

    for dir in &dirs_to_create {
        if let Some(dst_client) = &dst_conn {
            let mut current = PathBuf::new();
            for component in dir.components() {
                current.push(component);
                let _ = dst_client.create_dir(&current);
            }
        } else {
            let _ = std::fs::create_dir_all(dir);
        }
    }

    let mut results = TransferResults::default();
    let mut bytes_copied_acc = 0u64;

    if file_mappings.is_empty() {
        let _ = control.event_tx.send(TransferEvent::JobCompleted {
            job_id: control.job_id,
            results: results.clone(),
        });
        return Ok(results);
    }

    for (idx, (src, dst, size)) in file_mappings.iter().enumerate() {
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }
        control.wait_if_paused();
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }

        let start = Instant::now();
        let _ = control.event_tx.send(TransferEvent::FileStarted {
            job_id: control.job_id,
            file: src.clone(),
            index: idx,
        });

        let copy_res = (|| -> anyhow::Result<()> {
            let mut reader: Box<dyn Read + Send> = if let Some(src_conn) = &src_conn {
                let client = src_conn
                    .0
                    .lock()
                    .map_err(|_| anyhow!(t("error_mutex_poisoned")))?;
                let file = client.sftp.open(src)?;
                Box::new(file)
            } else {
                Box::new(std::fs::File::open(src)?)
            };

            let mut writer: Box<dyn Write + Send> = if let Some(dst_conn) = &dst_conn {
                let client = dst_conn
                    .0
                    .lock()
                    .map_err(|_| anyhow!(t("error_mutex_poisoned")))?;
                let file = client.sftp.create(dst)?;
                Box::new(file)
            } else {
                if let Some(parent) = dst.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                Box::new(std::fs::File::create(dst)?)
            };

            let mut buffer = vec![0u8; 64 * 1024];
            let mut file_bytes = 0u64;
            loop {
                if control.cancelled() {
                    return Err(anyhow!("Job cancelled"));
                }
                let n = reader.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                writer.write_all(&buffer[..n])?;
                file_bytes += n as u64;
                let _ = control.event_tx.send(TransferEvent::FileProgress {
                    job_id: control.job_id,
                    bytes_copied: bytes_copied_acc + file_bytes,
                    bytes_total: total_bytes,
                });
            }
            bytes_copied_acc += file_bytes;
            Ok(())
        })();

        match copy_res {
            Ok(()) => {
                let result = FileTransferResult {
                    src: src.clone(),
                    dst: dst.clone(),
                    size: *size,
                    src_hash: None,
                    dst_hash: None,
                    verified: true,
                    duration: start.elapsed(),
                };
                results.completed_files.push(result.clone());
                let _ = control.event_tx.send(TransferEvent::FileCompleted {
                    job_id: control.job_id,
                    result,
                });
            }
            Err(e) => {
                let msg = t("error_copying_to")
                    .replacen("{}", &src.to_string_lossy(), 1)
                    .replacen("{}", &dst.to_string_lossy(), 1)
                    .replacen("{}", &e.to_string(), 1);
                let failed = FailedFile {
                    src: src.clone(),
                    dst: dst.clone(),
                    error: msg.clone(),
                    retries: 0,
                };
                results.failed_files.push(failed.clone());
                let _ = control.event_tx.send(TransferEvent::FileFailed {
                    job_id: control.job_id,
                    error: failed,
                });
                let _ = control.event_tx.send(TransferEvent::JobFailed {
                    job_id: control.job_id,
                    error: msg.clone(),
                });
                return Err(anyhow!(msg));
            }
        }
    }

    if is_move {
        for src in &sources {
            if control.cancelled() {
                break;
            }
            if let Some(src_client) = &src_conn {
                if let Err(e) = src_client.delete_recursive(src) {
                    let msg = t("error_remote_source_delete_failed")
                        .replacen("{}", &src.to_string_lossy(), 1)
                        .replacen("{}", &e.to_string(), 1);
                    results.failed_files.push(FailedFile {
                        src: src.clone(),
                        dst: PathBuf::new(),
                        error: msg.clone(),
                        retries: 0,
                    });
                    return Err(anyhow!(msg));
                }
            } else if let Err(e) = delete_recursive(src) {
                let msg = t("error_delete_source_failed")
                    .replacen("{}", &src.to_string_lossy(), 1)
                    .replacen("{}", &e.to_string(), 1);
                results.failed_files.push(FailedFile {
                    src: src.clone(),
                    dst: PathBuf::new(),
                    error: msg.clone(),
                    retries: 0,
                });
                return Err(anyhow!(msg));
            }
        }
    }

    let _ = control.event_tx.send(TransferEvent::JobCompleted {
        job_id: control.job_id,
        results: results.clone(),
    });
    Ok(results)
}

async fn fast_remote_rename(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
    src_client: &SharedSshClient,
    dst_conn: &Option<SharedSshClient>,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    let is_dir_for_conn = |path: &Path, conn: &Option<SharedSshClient>| -> bool {
        if let Some(client) = conn {
            if let Ok(c) = client.0.lock()
                && let Ok(stat) = c.sftp.stat(path)
            {
                return stat.is_dir();
            }
            false
        } else {
            path.is_dir()
        }
    };

    let total_files = sources.len();
    let _ = control.event_tx.send(TransferEvent::ScanComplete {
        job_id: control.job_id,
        total_files,
        total_bytes: 0,
    });

    let mut results = TransferResults::default();
    for (idx, src) in sources.iter().enumerate() {
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }
        let name = src
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let dst = if is_destination_parent_dir(&sources, &destination_dir, |p| {
            is_dir_for_conn(p, dst_conn)
        }) {
            destination_dir.join(&name)
        } else {
            destination_dir.clone()
        };

        let start = Instant::now();
        let _ = control.event_tx.send(TransferEvent::FileStarted {
            job_id: control.job_id,
            file: src.clone(),
            index: idx,
        });

        if let Err(e) = src_client.rename_move(src, &dst) {
            let err_msg = t("error_remote_move_failed").replacen("{}", &e.to_string(), 1);
            let failed = FailedFile {
                src: src.clone(),
                dst: dst.clone(),
                error: err_msg.clone(),
                retries: 0,
            };
            results.failed_files.push(failed.clone());
            let _ = control.event_tx.send(TransferEvent::FileFailed {
                job_id: control.job_id,
                error: failed,
            });
            return Err(anyhow!(err_msg));
        }

        let result = FileTransferResult {
            src: src.clone(),
            dst,
            size: 0,
            src_hash: None,
            dst_hash: None,
            verified: true,
            duration: start.elapsed(),
        };
        results.completed_files.push(result.clone());
        let _ = control.event_tx.send(TransferEvent::FileCompleted {
            job_id: control.job_id,
            result,
        });
    }

    let _ = control.event_tx.send(TransferEvent::JobCompleted {
        job_id: control.job_id,
        results: results.clone(),
    });
    Ok(results)
}
