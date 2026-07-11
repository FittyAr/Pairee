use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use anyhow::anyhow;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::job::{
    FileTransferResult, FailedFile, SkippedFile, TransferResults, TransferOperation,
};
use super::options::TransferOptions;
use super::events::TransferEvent;
use super::filter::TransferFilter;
use super::conflict::resolve_filename_conflict;
use super::pipeline::copy_file_pipelined;
use super::metadata::preserve_metadata;

pub struct TransferWorker {
    pub job_id: Uuid,
    pub operation: TransferOperation,
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub options: TransferOptions,
    pub is_paused: Arc<AtomicBool>,
    pub is_cancelled: Arc<AtomicBool>,
    pub skip_file_flag: Arc<AtomicBool>,
    pub event_tx: mpsc::UnboundedSender<TransferEvent>,
    pub active_conflict: Arc<std::sync::Mutex<Option<crate::fs::transfer::conflict::ConflictResolution>>>,
}

impl TransferWorker {
    pub fn new(
        job_id: Uuid,
        operation: TransferOperation,
        sources: Vec<PathBuf>,
        destination: PathBuf,
        options: TransferOptions,
        is_paused: Arc<AtomicBool>,
        is_cancelled: Arc<AtomicBool>,
        skip_file_flag: Arc<AtomicBool>,
        event_tx: mpsc::UnboundedSender<TransferEvent>,
        active_conflict: Arc<std::sync::Mutex<Option<crate::fs::transfer::conflict::ConflictResolution>>>,
    ) -> Self {
        Self {
            job_id,
            operation,
            sources,
            destination,
            options,
            is_paused,
            is_cancelled,
            skip_file_flag,
            event_tx,
            active_conflict,
        }
    }

    pub async fn run(self) -> Result<TransferResults, anyhow::Error> {
        let mut auto_resolution = None;
        let _ = self.event_tx.send(TransferEvent::JobStarted { job_id: self.job_id });

        // Detección LAN y optimización de buffers
        let is_lan = super::network::is_lan_path(&self.destination);
        let mut options = self.options.clone();
        if is_lan {
            options.buffer_size = crate::fs::transfer::options::BufferSize::_4MB;
        }

        // --- FASE 1: ESCANEO ---
        let _ = self.event_tx.send(TransferEvent::ScanProgress {
            job_id: self.job_id,
            files_found: 0,
        });

        let mut scan_mappings = Vec::new();
        let mut total_bytes = 0u64;
        let mut files_scanned = 0usize;

        let filter = TransferFilter::parse(options.filter_mask.as_deref().unwrap_or(""));

        for src in &self.sources {
            if self.is_cancelled.load(Ordering::Relaxed) {
                return Err(anyhow!("Job cancelled during scan"));
            }

            if src.is_dir() && !(src.is_symlink() && !options.follow_symlinks) {
                let mut dirs_to_visit = VecDeque::new();
                dirs_to_visit.push_back(src.clone());

                while let Some(dir) = dirs_to_visit.pop_front() {
                    if self.is_cancelled.load(Ordering::Relaxed) {
                        return Err(anyhow!("Job cancelled during scan"));
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
                            // No seguir el symlink -> encolarlo como archivo de tamaño 0
                            let size = 0u64;
                            if let Ok(rel) = path.strip_prefix(src) {
                                let folder_name = src.file_name().unwrap_or_default();
                                let dst_path = self.destination.join(folder_name).join(rel);
                                scan_mappings.push((path, dst_path, size));
                                files_scanned += 1;
                            }
                            continue;
                        }

                        if path.is_dir() {
                            dirs_to_visit.push_back(path);
                        } else {
                            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                            
                            // Aplicar filtros
                            if !filter.matches(&path, size) {
                                continue;
                            }

                            // Determinar destino relativo
                            if let Ok(rel) = path.strip_prefix(src) {
                                let folder_name = src.file_name().unwrap_or_default();
                                let dst_path = self.destination.join(folder_name).join(rel);
                                scan_mappings.push((path, dst_path, size));
                                total_bytes += size;
                                files_scanned += 1;
                            }
                        }
                    }

                    let _ = self.event_tx.send(TransferEvent::ScanProgress {
                        job_id: self.job_id,
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

                let file_name = src.file_name().unwrap_or_default();
                let dst_path = self.destination.join(file_name);
                scan_mappings.push((src.clone(), dst_path, size));
                total_bytes += size;
                files_scanned += 1;

                let _ = self.event_tx.send(TransferEvent::ScanProgress {
                    job_id: self.job_id,
                    files_found: files_scanned,
                });
            }
        }

        let _ = self.event_tx.send(TransferEvent::ScanComplete {
            job_id: self.job_id,
            total_files: files_scanned,
            total_bytes,
        });

        // Verificar espacio libre en destino
        if let Ok(free_space) = super::network::get_free_space(&self.destination) {
            if free_space < total_bytes {
                let _ = self.event_tx.send(TransferEvent::FileSkipped {
                    job_id: self.job_id,
                    file: self.destination.clone(),
                    reason: format!(
                        "Warning: Low disk space. Required: {}, Available: {}",
                        bytesize::ByteSize(total_bytes),
                        bytesize::ByteSize(free_space)
                    ),
                });
            }
        }

        // --- FASE 2: TRANSFERENCIA ---
        let mut results = TransferResults::default();
        let bytes_transferred_acc = Arc::new(AtomicU64::new(0));

        // Spawn de tarea para reportar velocidad y ETA periódicos
        let event_tx_speed = self.event_tx.clone();
        let job_id_speed = self.job_id;
        let bytes_acc_speed = Arc::clone(&bytes_transferred_acc);
        let is_cancelled_speed = Arc::clone(&self.is_cancelled);
        
        let _speed_reporter = tokio::spawn(async move {
            let mut last_bytes = 0u64;
            let mut interval = tokio::time::interval(Duration::from_secs(1));

            loop {
                interval.tick().await;
                if is_cancelled_speed.load(Ordering::Relaxed) {
                    break;
                }

                let current_bytes = bytes_acc_speed.load(Ordering::SeqCst);
                let delta = current_bytes.saturating_sub(last_bytes);
                last_bytes = current_bytes;

                let bytes_per_second = delta as f64;
                
                let remaining_bytes = total_bytes.saturating_sub(current_bytes);
                let eta_seconds = if bytes_per_second > 0.0 {
                    Some((remaining_bytes as f64 / bytes_per_second) as u64)
                } else {
                    None
                };

                let _ = event_tx_speed.send(TransferEvent::SpeedUpdate {
                    job_id: job_id_speed,
                    bytes_per_second,
                    eta_seconds,
                });

                if current_bytes >= total_bytes && total_bytes > 0 {
                    break;
                }
            }
        });

        for (idx, (src, mut dst, size)) in scan_mappings.into_iter().enumerate() {
            // Verificar cancelación
            if self.is_cancelled.load(Ordering::Relaxed) {
                return Err(anyhow!("Job cancelled"));
            }

            // Verificar pausa
            while self.is_paused.load(Ordering::Relaxed) {
                if self.is_cancelled.load(Ordering::Relaxed) {
                    return Err(anyhow!("Job cancelled"));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            // Verificar si el usuario pidió omitir este archivo individual
            if self.skip_file_flag.swap(false, Ordering::Relaxed) {
                results.skipped_files.push(SkippedFile {
                    src: src.clone(),
                    reason: "Skipped by user".to_string(),
                });
                let _ = self.event_tx.send(TransferEvent::FileSkipped {
                    job_id: self.job_id,
                    file: src.clone(),
                    reason: "Skipped by user".to_string(),
                });
                continue;
            }

            // Manejar conflicto si existe
            if dst.exists() {
                let mut resolution = options.conflict_resolution.clone();
                if resolution == "ask" {
                    let chosen = if let Some(auto_res) = auto_resolution {
                        auto_res
                    } else {
                        // Notificar conflicto
                        let _ = self.event_tx.send(TransferEvent::ConflictDetected {
                            job_id: self.job_id,
                            file: dst.clone(),
                            conflict: crate::fs::transfer::conflict::ConflictInfo {
                                src_path: src.clone(),
                                dst_path: dst.clone(),
                                src_size: src.metadata().map(|m| m.len()).unwrap_or(0),
                                dst_size: dst.metadata().map(|m| m.len()).unwrap_or(0),
                                src_modified: src.metadata().and_then(|m| m.modified()).ok(),
                                dst_modified: dst.metadata().and_then(|m| m.modified()).ok(),
                            },
                        });

                        // Limpiar conflicto anterior y esperar respuesta de la UI
                        {
                            let mut guard = self.active_conflict.lock().unwrap();
                            *guard = None;
                        }

                        while self.active_conflict.lock().unwrap().is_none() {
                            if self.is_cancelled.load(Ordering::Relaxed) {
                                return Err(anyhow!("Job cancelled"));
                            }
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }

                        let ch = self.active_conflict.lock().unwrap().clone().unwrap_or(crate::fs::transfer::conflict::ConflictResolution::Skip);
                        match ch {
                            crate::fs::transfer::conflict::ConflictResolution::OverwriteAll |
                            crate::fs::transfer::conflict::ConflictResolution::OverwriteOlderAll |
                            crate::fs::transfer::conflict::ConflictResolution::SkipAll |
                            crate::fs::transfer::conflict::ConflictResolution::RenameAll => {
                                auto_resolution = Some(ch);
                            }
                            _ => {}
                        }
                        ch
                    };

                    resolution = match chosen {
                        crate::fs::transfer::conflict::ConflictResolution::Overwrite | crate::fs::transfer::conflict::ConflictResolution::OverwriteAll => "overwrite".to_string(),
                        crate::fs::transfer::conflict::ConflictResolution::OverwriteOlder | crate::fs::transfer::conflict::ConflictResolution::OverwriteOlderAll => "overwrite_older".to_string(),
                        crate::fs::transfer::conflict::ConflictResolution::Rename | crate::fs::transfer::conflict::ConflictResolution::RenameAll | crate::fs::transfer::conflict::ConflictResolution::KeepBoth => "rename".to_string(),
                        crate::fs::transfer::conflict::ConflictResolution::Cancel => {
                            self.is_cancelled.store(true, Ordering::SeqCst);
                            return Err(anyhow!("Job cancelled"));
                        }
                        _ => "skip".to_string(),
                    };
                }

                match resolution.as_str() {
                    "skip" => {
                        results.skipped_files.push(SkippedFile {
                            src: src.clone(),
                            reason: "File already exists (skipped)".to_string(),
                        });
                        let _ = self.event_tx.send(TransferEvent::FileSkipped {
                            job_id: self.job_id,
                            file: src.clone(),
                            reason: "File already exists".to_string(),
                        });
                        continue;
                    }
                    "rename" | "keep_both" => {
                        dst = resolve_filename_conflict(&dst);
                    }
                    "overwrite_older" => {
                        let src_time = src.metadata().and_then(|m| m.modified()).ok();
                        let dst_time = dst.metadata().and_then(|m| m.modified()).ok();
                        if let (Some(s_time), Some(d_time)) = (src_time, dst_time) {
                            if s_time <= d_time {
                                // Omitir, el destino es más nuevo o igual
                                results.skipped_files.push(SkippedFile {
                                    src: src.clone(),
                                    reason: "Destination is newer or equal (skipped)".to_string(),
                                });
                                let _ = self.event_tx.send(TransferEvent::FileSkipped {
                                    job_id: self.job_id,
                                    file: src.clone(),
                                    reason: "Destination is newer or equal".to_string(),
                                });
                                continue;
                            }
                        }
                    }
                    _ => {} // Overwrite
                }
            }

            let _ = self.event_tx.send(TransferEvent::FileStarted {
                job_id: self.job_id,
                file: src.clone(),
                index: idx,
            });

            // Fase Copia / Transferencia con reintentos
            let mut retries = 0u32;
            let mut copy_success = false;
            let mut last_error = String::new();
            let mut src_hash = None;
            let mut dst_hash = None;
            let file_start = Instant::now();

            let is_symlink = src.is_symlink();
            let recreate_link = is_symlink && !options.follow_symlinks;

            if recreate_link {
                match (|| -> std::io::Result<()> {
                    let target = std::fs::read_link(&src)?;
                    #[cfg(target_os = "windows")]
                    {
                        let absolute_target = if target.is_relative() {
                            src.parent().map(|p| p.join(&target)).unwrap_or_else(|| target.clone())
                        } else {
                            target.clone()
                        };
                        let is_dir = absolute_target.is_dir();
                        if dst.exists() {
                            let _ = std::fs::remove_file(&dst);
                            let _ = std::fs::remove_dir_all(&dst);
                        }
                        if is_dir {
                            std::os::windows::fs::symlink_dir(&target, &dst)?;
                        } else {
                            std::os::windows::fs::symlink_file(&target, &dst)?;
                        }
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        if dst.exists() {
                            let _ = std::fs::remove_file(&dst);
                        }
                        std::os::unix::fs::symlink(&target, &dst)?;
                    }
                    Ok(())
                })() {
                    Ok(_) => {
                        copy_success = true;
                    }
                    Err(e) => {
                        last_error = format!("Error creating symlink: {}", e);
                    }
                }
            } else {
                while retries <= options.max_retries {
                    if self.is_cancelled.load(Ordering::Relaxed) {
                        return Err(anyhow!("Job cancelled"));
                    }

                    match copy_file_pipelined(
                        &src,
                        &dst,
                        &options,
                        &self.event_tx,
                        self.job_id,
                        Arc::clone(&self.is_paused),
                        Arc::clone(&self.is_cancelled),
                        Arc::clone(&bytes_transferred_acc),
                    )
                    .await
                    {
                        Ok((s_hash, d_hash)) => {
                            src_hash = s_hash;
                            dst_hash = d_hash;
                            copy_success = true;
                            break;
                        }
                        Err(e) => {
                            retries += 1;
                            last_error = e.to_string();
                            if retries <= options.max_retries {
                                // Backoff exponencial simple: 100ms, 200ms, 400ms...
                                let backoff = Duration::from_millis(100 * (1 << retries));
                                tokio::time::sleep(backoff).await;
                            }
                        }
                    }
                }
            }

            if !copy_success {
                results.failed_files.push(FailedFile {
                    src: src.clone(),
                    dst: dst.clone(),
                    error: last_error.clone(),
                    retries,
                });
                let _ = self.event_tx.send(TransferEvent::FileFailed {
                    job_id: self.job_id,
                    error: FailedFile {
                        src: src.clone(),
                        dst: dst.clone(),
                        error: last_error.clone(),
                        retries,
                    },
                });
                if options.halt_on_error {
                    let _ = self.event_tx.send(TransferEvent::JobFailed {
                        job_id: self.job_id,
                        error: format!("Halt on error triggered by file failure: {}", last_error),
                    });
                    return Err(anyhow::anyhow!("Halt on error: {}", last_error));
                }
                continue;
            }

            // Preservar metadatos
            let _ = preserve_metadata(&src, &dst, &options);

            // Verificación del hash
            let verified = true;
            if options.verify_after_copy {
                let _ = self.event_tx.send(TransferEvent::VerifyStarted {
                    job_id: self.job_id,
                    file: src.clone(),
                    algorithm: options.hash_algorithm.as_str().to_string(),
                });

                if let (Some(sh), Some(dh)) = (src_hash.as_ref(), dst_hash.as_ref()) {
                    let _ = self.event_tx.send(TransferEvent::VerifyProgress {
                        job_id: self.job_id,
                        bytes_verified: size,
                        bytes_total: size,
                    });

                    if sh != dh {
                        results.failed_files.push(FailedFile {
                            src: src.clone(),
                            dst: dst.clone(),
                            error: "Hash verification mismatch".to_string(),
                            retries: 0,
                        });
                        let _ = self.event_tx.send(TransferEvent::FileFailed {
                            job_id: self.job_id,
                            error: FailedFile {
                                src: src.clone(),
                                dst: dst.clone(),
                                error: "Hash verification mismatch".to_string(),
                                retries: 0,
                            },
                        });
                        if options.halt_on_error {
                            let _ = self.event_tx.send(TransferEvent::JobFailed {
                                job_id: self.job_id,
                                error: "Halt on error triggered by hash mismatch".to_string(),
                            });
                            return Err(anyhow::anyhow!("Halt on error: Hash mismatch"));
                        }
                        continue;
                    }
                }
            }

            // Si la operación es MOVE y fue verificado con éxito, eliminar origen
            if self.operation == TransferOperation::Move && verified {
                let _ = std::fs::remove_file(&src);
            }

            let file_result = FileTransferResult {
                src: src.clone(),
                dst: dst.clone(),
                size,
                src_hash: src_hash.clone(),
                dst_hash: dst_hash.clone(),
                verified,
                duration: file_start.elapsed(),
            };

            results.completed_files.push(file_result.clone());
            
            let _ = self.event_tx.send(TransferEvent::FileCompleted {
                job_id: self.job_id,
                result: file_result,
            });
        }

        let _ = self.event_tx.send(TransferEvent::JobCompleted {
            job_id: self.job_id,
            results: results.clone(),
        });

        Ok(results)
    }
}
