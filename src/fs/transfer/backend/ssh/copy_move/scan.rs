use super::super::super::super::events::TransferEvent;
use super::super::super::super::worker::is_destination_parent_dir;
use super::super::super::BackendControl;
use crate::fs::ssh::SharedSshClient;
use anyhow::anyhow;
use std::path::{Path, PathBuf};

pub struct ScanOutput {
    pub total_bytes: u64,
    pub file_mappings: Vec<(PathBuf, PathBuf, u64)>,
    pub dirs_to_create: Vec<PathBuf>,
}

pub fn scan_sources(
    sources: &[PathBuf],
    destination_dir: &Path,
    src_conn: &Option<SharedSshClient>,
    dst_conn: &Option<SharedSshClient>,
    control: &BackendControl,
) -> Result<ScanOutput, anyhow::Error> {
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

    let destination_dir_is_dir =
        is_destination_parent_dir(sources, destination_dir, |p| is_dir_for_conn(p, dst_conn));

    for src in sources {
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }
        let is_dir = is_dir_for_conn(src, src_conn);
        let name = src
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        if is_dir {
            let base_dst = if destination_dir_is_dir {
                destination_dir.join(&name)
            } else {
                destination_dir.to_path_buf()
            };
            dirs_to_create.push(base_dst.clone());

            if let Some(src_client) = src_conn {
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
            let size = if let Some(src_client) = src_conn {
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
                destination_dir.to_path_buf()
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

    Ok(ScanOutput {
        total_bytes,
        file_mappings,
        dirs_to_create,
    })
}
