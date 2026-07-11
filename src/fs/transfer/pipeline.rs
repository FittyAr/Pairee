use std::io::{Read, Write};
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
    
    // Abrir archivo origen
    let mut src_file = super::direct_io::open_reader_direct(src, options.direct_io)
        .map_err(|e| anyhow!("Error opening source file: {}", e))?;
    let metadata = src_file.metadata()?;
    let file_size = metadata.len();
    
    // Crear archivo destino y asegurar directorios
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut dst_file = super::direct_io::open_writer_direct(dst, options.direct_io)
        .map_err(|e| anyhow!("Error creating destination file: {}", e))?;

    // Configurar hashers si la verificación está activa
    let src_hasher: Option<Box<dyn HashStrategy>> = if options.verify_after_copy {
        Some(create_hasher(options.hash_algorithm))
    } else {
        None
    };
    
    let dst_hasher_algo = options.hash_algorithm;
    let verify_active = options.verify_after_copy;

    // Crear un canal mpsc para los bloques de datos con un límite de capacidad
    // para evitar que el lector llene la memoria si la escritura es lenta (backpressure)
    let (block_tx, mut block_rx) = mpsc::channel::<Vec<u8>>(4);

    // Clonar flags de control
    let is_paused_reader = Arc::clone(&is_paused);
    let is_cancelled_reader = Arc::clone(&is_cancelled);
    
    // --- ETAPA 1: Hilo Lector ---
    let reader_handle = tokio::task::spawn_blocking(move || -> Result<Option<String>, anyhow::Error> {
        let mut buffer = vec![0; buffer_bytes];
        let mut hasher = src_hasher;
        
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

            let bytes_read = src_file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            let chunk = buffer[..bytes_read].to_vec();
            
            // Actualizar hash del origen
            if let Some(ref mut h) = hasher {
                h.update(&chunk);
            }

            // Enviar bloque al escritor (bloqueante si el canal está lleno)
            if block_tx.blocking_send(chunk).is_err() {
                return Err(anyhow!("Writer thread disconnected"));
            }
        }

        let hash_result = hasher.map(|h| h.finalize());
        Ok(hash_result)
    });

    // --- ETAPA 2: Hilo Escritor (Consumidor) ---
    let is_paused_writer = Arc::clone(&is_paused);
    let is_cancelled_writer = Arc::clone(&is_cancelled);
    let event_tx_writer = event_tx.clone();
    
    let writer_handle = tokio::task::spawn_blocking(move || -> Result<(u64, Option<String>), anyhow::Error> {
        let mut hasher = if verify_active {
            Some(create_hasher(dst_hasher_algo))
        } else {
            None
        };
        
        let mut bytes_written_total = 0u64;
        let mut last_progress_sent = Instant::now();
        let progress_interval = Duration::from_millis(150);

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

            dst_file.write_all(&chunk)?;
            bytes_written_total += chunk.len() as u64;

            // Actualizar hash del destino
            if let Some(ref mut h) = hasher {
                h.update(&chunk);
            }

            // Actualizar progreso global de bytes
            bytes_transferred_acc.fetch_add(chunk.len() as u64, Ordering::SeqCst);

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

        // Forzar flush a disco
        dst_file.sync_all()?;
        
        let hash_result = hasher.map(|h| h.finalize());
        Ok((bytes_written_total, hash_result))
    });

    // Esperar a que ambas etapas terminen
    let (reader_res, writer_res) = tokio::join!(reader_handle, writer_handle);

    let src_hash = reader_res.map_err(|e| anyhow!("Reader task join error: {}", e))??;
    let (_bytes_written, dst_hash) = writer_res.map_err(|e| anyhow!("Writer task join error: {}", e))??;

    Ok((src_hash, dst_hash))
}
