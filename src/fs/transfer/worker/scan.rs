use anyhow::anyhow;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::super::events::TransferEvent;
use super::super::filter::TransferFilter;
use super::super::job::TransferOperation;
use super::super::options::TransferOptions;
use super::destination::is_destination_parent_dir;

/// Result of the scan phase: file mappings and aggregate totals.
pub(super) struct ScanOutcome {
    pub mappings: Vec<(PathBuf, PathBuf, u64)>,
    pub dirs_to_delete: Vec<PathBuf>,
    pub total_bytes: u64,
    // Part of phase outcome API; ScanComplete is already emitted during scan.
    #[allow(dead_code)]
    pub files_scanned: usize,
}

/// Scan sources and build destination mappings for the transfer.
pub(super) fn scan(
    sources: &[PathBuf],
    destination: &std::path::Path,
    operation: TransferOperation,
    options: &TransferOptions,
    job_id: Uuid,
    is_cancelled: &AtomicBool,
    event_tx: &mpsc::UnboundedSender<TransferEvent>,
) -> Result<ScanOutcome, anyhow::Error> {
    let _ = event_tx.send(TransferEvent::ScanProgress {
        job_id,
        files_found: 0,
    });

    let mut scan_mappings = Vec::new();
    let mut dirs_to_delete = Vec::new();
    let mut total_bytes = 0u64;
    let mut files_scanned = 0usize;

    let filter = TransferFilter::parse(options.filter_mask.as_deref().unwrap_or(""));

    let is_parent_dir = is_destination_parent_dir(sources, destination, |p| p.is_dir());

    for src in sources {
        if is_cancelled.load(Ordering::Relaxed) {
            return Err(anyhow!("Job cancelled during scan"));
        }

        if src.is_dir() && (!src.is_symlink() || options.follow_symlinks) {
            let base_dst = if is_parent_dir {
                let folder_name = src.file_name().unwrap_or_default();
                destination.join(folder_name)
            } else {
                destination.to_path_buf()
            };

            let mut dirs_to_visit = VecDeque::new();
            dirs_to_visit.push_back(src.clone());
            if operation == TransferOperation::Delete || operation == TransferOperation::Move {
                dirs_to_delete.push(src.clone());
            }

            while let Some(dir) = dirs_to_visit.pop_front() {
                if is_cancelled.load(Ordering::Relaxed) {
                    return Err(anyhow!("Job cancelled during scan"));
                }

                if operation == TransferOperation::Copy || operation == TransferOperation::Move {
                    if let Ok(rel) = dir.strip_prefix(src) {
                        let dst_dir = base_dst.join(rel);
                        let _ = std::fs::create_dir_all(&dst_dir);
                    }
                }

                let entries = match std::fs::read_dir(&dir) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                for entry in entries.flatten() {
                    let path = entry.path();
                    let is_symlink = path.is_symlink();

                    if is_symlink && options.skip_symlinks {
                        continue;
                    }

                    if is_symlink && !options.follow_symlinks {
                        let size = 0u64;
                        if operation == TransferOperation::Delete {
                            scan_mappings.push((path, PathBuf::new(), size));
                            files_scanned += 1;
                        } else if let Ok(rel) = path.strip_prefix(src) {
                            let dst_path = base_dst.join(rel);
                            scan_mappings.push((path, dst_path, size));
                            files_scanned += 1;
                        }
                        continue;
                    }

                    if path.is_dir() {
                        dirs_to_visit.push_back(path.clone());
                        if operation == TransferOperation::Delete
                            || operation == TransferOperation::Move
                        {
                            dirs_to_delete.push(path);
                        }
                    } else {
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

                        if !filter.matches(&path, size) {
                            continue;
                        }

                        if operation == TransferOperation::Delete {
                            scan_mappings.push((path, PathBuf::new(), size));
                            total_bytes += size;
                            files_scanned += 1;
                        } else if let Ok(rel) = path.strip_prefix(src) {
                            let dst_path = base_dst.join(rel);
                            scan_mappings.push((path, dst_path, size));
                            total_bytes += size;
                            files_scanned += 1;
                        }
                    }
                }

                let _ = event_tx.send(TransferEvent::ScanProgress {
                    job_id,
                    files_found: files_scanned,
                });
            }
        } else {
            let is_symlink = src.is_symlink();
            if is_symlink && options.skip_symlinks {
                continue;
            }

            let size = if is_symlink && !options.follow_symlinks {
                0
            } else {
                src.metadata().map(|m| m.len()).unwrap_or(0)
            };

            if !filter.matches(src, size) {
                continue;
            }

            if operation == TransferOperation::Delete {
                scan_mappings.push((src.clone(), PathBuf::new(), size));
                total_bytes += size;
                files_scanned += 1;
            } else {
                let dst_path = if is_parent_dir {
                    let file_name = src.file_name().unwrap_or_default();
                    destination.join(file_name)
                } else {
                    destination.to_path_buf()
                };
                scan_mappings.push((src.clone(), dst_path, size));
                total_bytes += size;
                files_scanned += 1;
            }

            let _ = event_tx.send(TransferEvent::ScanProgress {
                job_id,
                files_found: files_scanned,
            });
        }
    }

    let _ = event_tx.send(TransferEvent::ScanComplete {
        job_id,
        total_files: files_scanned,
        total_bytes,
    });

    Ok(ScanOutcome {
        mappings: scan_mappings,
        dirs_to_delete,
        total_bytes,
        files_scanned,
    })
}
