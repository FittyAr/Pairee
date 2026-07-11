use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use anyhow::anyhow;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::options::TransferOptions;
use super::events::TransferEvent;
use super::hash::{create_hasher, HashStrategy};

/// Copia un archivo individual usando un pipeline de lectura y escritura en paralelo.
/// Retorna `Ok((Option<src_hash>, Option<dst_hash>))` si la transferencia fue exitosa.
pub async fn copy_file_pipelined(
    src: &Path,
    dst: &Path,
    options: &TransferOptions,
    event_tx: &mpsc::UnboundedSender<TransferEvent>,
    job_id: Uuid,
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
    bytes_transferred_acc: Arc<std::sync::atomic::AtomicU64>,
) -> Result<(Option<String>, Option<String>), anyhow::Error> {
    
    let buffer_bytes = options.buffer_size.to_bytes();
    let normalized_src = super::direct_io::to_long_path(src);
    let normalized_dst = super::direct_io::to_long_path(dst);
    
    // Obtener tamaño para lógica de progreso y de Direct I/O
    let file_size = std::fs::metadata(&normalized_src)?.len();
    
    // Asegurar directorios de destino
    if let Some(parent) = normalized_dst.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Configurar hashers si la verificación está activa
    let src_hasher: Option<Box<dyn HashStrategy>> = if options.verify_after_copy {
        Some(create_hasher(options.hash_algorithm))
    } else {
        None
    };
    
    let dst_hasher_algo = options.hash_algorithm;
    let verify_active = options.verify_after_copy;

    // Crear un canal mpsc para los bloques de datos con un límite de capacidad (backpressure)
    let (block_tx, mut block_rx) = mpsc::channel::<Vec<u8>>(4);

    // Clonar flags de control
    let is_paused_reader = Arc::clone(&is_paused);
    let is_cancelled_reader = Arc::clone(&is_cancelled);
    let options_reader = options.clone();
    
    // --- ETAPA 1: Hilo Lector ---
    let reader_handle = tokio::task::spawn_blocking(move || -> Result<Option<String>, anyhow::Error> {
        let mut aligned_buf = super::direct_io::AlignedBuffer::new(buffer_bytes, 4096);
        let mut hasher = src_hasher;
        
        let mut use_direct_io = options_reader.direct_io;
        let mut src_file_std = std::fs::File::open(&normalized_src)
            .map_err(|e| anyhow!("Error opening source file: {}", e))?;
        
        let mut src_file_direct = if use_direct_io {
            match super::direct_io::open_reader_direct(&normalized_src, true) {
                Ok(f) => Some(f),
                Err(_) => {
                    use_direct_io = false;
                    None
                }
            }
        } else {
            None
        };

        let start_time = Instant::now();
        let mut total_bytes_read = 0u64;
        let mut offset = 0u64;

        loop {
            // Verificar cancelación
            if is_cancelled_reader.load(Ordering::Relaxed) {
                return Err(anyhow!("Transfer cancelled"));
            }

            // Verificar pausa
            while is_paused_reader.load(Ordering::Relaxed) {
                if is_cancelled_reader.load(Ordering::Relaxed) {
                    return Err(anyhow!("Transfer cancelled"));
                }
                std::thread::sleep(Duration::from_millis(50));
            }

            // Lógica de lectura con fallback y alineación
            let bytes_read = if use_direct_io {
                let remaining = file_size.saturating_sub(offset);
                if remaining < 4096 || remaining % 4096 != 0 {
                    // Último bloque parcial desalineado -> usar I/O estándar
                    use_direct_io = false;
                    src_file_std.seek(SeekFrom::Start(offset))?;
                    src_file_std.read(aligned_buf.as_mut_slice())?
                } else {
                    let direct_f = src_file_direct.as_mut().unwrap();
                    match direct_f.read(aligned_buf.as_mut_slice()) {
                        Ok(n) => n,
                        Err(_) => {
                            // Fallback transparente a estándar
                            use_direct_io = false;
                            src_file_std.seek(SeekFrom::Start(offset))?;
                            src_file_std.read(aligned_buf.as_mut_slice())?
                        }
                    }
                }
            } else {
                src_file_std.read(aligned_buf.as_mut_slice())?
            };

            if bytes_read == 0 {
                break;
            }

            let chunk = aligned_buf.as_slice()[..bytes_read].to_vec();
            
            // Actualizar hash del origen
            if let Some(ref mut h) = hasher {
                h.update(&chunk);
            }

            // Enviar bloque al escritor
            if block_tx.blocking_send(chunk).is_err() {
                return Err(anyhow!("Writer thread disconnected"));
            }

            offset += bytes_read as u64;
            total_bytes_read += bytes_read as u64;

            // Throttling: Limitar ancho de banda
            if let Some(rate) = options_reader.limit_bandwidth_rate {
                if rate > 0 {
                    let expected_duration = Duration::from_secs_f64(total_bytes_read as f64 / rate as f64);
                    let actual_duration = start_time.elapsed();
                    if actual_duration < expected_duration {
                        let sleep_dur = expected_duration - actual_duration;
                        std::thread::sleep(sleep_dur);
                    }
                }
            }
        }

        let hash_result = hasher.map(|h| h.finalize());
        Ok(hash_result)
    });

    // --- ETAPA 2: Hilo Escritor (Consumidor) ---
    let is_paused_writer = Arc::clone(&is_paused);
    let is_cancelled_writer = Arc::clone(&is_cancelled);
    let event_tx_writer = event_tx.clone();
    let options_writer = options.clone();
    
    let writer_handle = tokio::task::spawn_blocking(move || -> Result<(u64, Option<String>), anyhow::Error> {
        let mut hasher = if verify_active {
            Some(create_hasher(dst_hasher_algo))
        } else {
            None
        };
        
        let mut use_direct_io = options_writer.direct_io;
        let mut dst_file_std = std::fs::File::create(&normalized_dst)
            .map_err(|e| anyhow!("Error creating destination file: {}", e))?;
        
        let mut dst_file_direct = if use_direct_io {
            match super::direct_io::open_writer_direct(&normalized_dst, true) {
                Ok(f) => Some(f),
                Err(_) => {
                    use_direct_io = false;
                    None
                }
            }
        } else {
            None
        };

        let mut aligned_writer_buf = if use_direct_io {
            Some(super::direct_io::AlignedBuffer::new(buffer_bytes, 4096))
        } else {
            None
        };

        let mut bytes_written_total = 0u64;
        let mut last_progress_sent = Instant::now();
        let progress_interval = Duration::from_millis(150);
        let mut offset = 0u64;

        while let Some(chunk) = block_rx.blocking_recv() {
            // Verificar cancelación
            if is_cancelled_writer.load(Ordering::Relaxed) {
                return Err(anyhow!("Transfer cancelled"));
            }

            // Verificar pausa
            while is_paused_writer.load(Ordering::Relaxed) {
                if is_cancelled_writer.load(Ordering::Relaxed) {
                    return Err(anyhow!("Transfer cancelled"));
                }
                std::thread::sleep(Duration::from_millis(50));
            }

            let chunk_len = chunk.len();

            // Lógica de escritura con buffer alineado y Direct I/O
            if use_direct_io {
                if chunk_len < 4096 || chunk_len % 4096 != 0 {
                    // Último bloque parcial desalineado -> usar I/O estándar
                    use_direct_io = false;
                    dst_file_std.seek(SeekFrom::Start(offset))?;
                    dst_file_std.write_all(&chunk)?;
                } else if let Some(ref mut aligned_buf) = aligned_writer_buf {
                    aligned_buf.as_mut_slice()[..chunk_len].copy_from_slice(&chunk);
                    let direct_f = dst_file_direct.as_mut().unwrap();
                    match direct_f.write_all(&aligned_buf.as_slice()[..chunk_len]) {
                        Ok(_) => {},
                        Err(_) => {
                            // Fallback
                            use_direct_io = false;
                            dst_file_std.seek(SeekFrom::Start(offset))?;
                            dst_file_std.write_all(&chunk)?;
                        }
                    }
                }
            } else {
                dst_file_std.write_all(&chunk)?;
            }

            bytes_written_total += chunk_len as u64;
            offset += chunk_len as u64;

            // Actualizar hash del destino
            if let Some(ref mut h) = hasher {
                h.update(&chunk);
            }

            // Actualizar progreso global de bytes
            bytes_transferred_acc.fetch_add(chunk_len as u64, Ordering::SeqCst);

            // Reportar progreso periódico
            if last_progress_sent.elapsed() >= progress_interval {
                last_progress_sent = Instant::now();
                let _ = event_tx_writer.send(TransferEvent::FileProgress {
                    job_id,
                    bytes_copied: bytes_written_total,
                    bytes_total: file_size,
                });
            }
        }

        // Sincronizar cambios a disco
        if use_direct_io {
            if let Some(f) = dst_file_direct {
                f.sync_all()?;
            }
        } else {
            dst_file_std.sync_all()?;
        }
        
        let hash_result = hasher.map(|h| h.finalize());
        Ok((bytes_written_total, hash_result))
    });

    // Esperar a que ambas etapas terminen
    let (reader_res, writer_res) = tokio::join!(reader_handle, writer_handle);

    let src_hash = reader_res.map_err(|e| anyhow!("Reader task join error: {}", e))??;
    let (_bytes_written, dst_hash) = writer_res.map_err(|e| anyhow!("Writer task join error: {}", e))??;

    Ok((src_hash, dst_hash))
}
