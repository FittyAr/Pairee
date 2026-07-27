use anyhow::anyhow;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::endpoint::TransferEndpoint;
use super::events::TransferEvent;
use super::hash::{HashStrategy, create_hasher};
use super::options::TransferOptions;

/// Copy a single file from `src` to `dst` using a parallel reader /
/// writer pipeline. The endpoints tell the function how to open
/// each side — `Local` for the OS filesystem, `Ssh` for an SFTP
/// session. The pipeline is endpoint-agnostic: hash streaming,
/// backpressure, bandwidth throttling and progress reporting are
/// the same regardless of the underlying transport.
///
/// Returns `Ok((Option<src_hash>, Option<dst_hash>))` if the
/// transfer was successful. The hashes are only computed when
/// `options.verify_after_copy` is `true`.
pub async fn copy_file_pipelined(
    src_endpoint: &TransferEndpoint,
    src: &Path,
    dst_endpoint: &TransferEndpoint,
    dst: &Path,
    options: &TransferOptions,
    event_tx: &mpsc::UnboundedSender<TransferEvent>,
    job_id: Uuid,
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
    bytes_transferred_acc: Arc<std::sync::atomic::AtomicU64>,
) -> Result<(Option<String>, Option<String>), anyhow::Error> {
    let buffer_bytes = options.buffer_size.to_bytes();

    // Direct I/O is a property of the underlying block device and
    // only makes sense when both endpoints are Local. For Ssh we
    // never try it (SFTP has no O_DIRECT equivalent).
    let local_direct_io = options.direct_io && src_endpoint.is_local() && dst_endpoint.is_local();

    // Total bytes for progress reporting. lstat follows the source
    // symlink; if `src` is a symlink, the worker should have handled
    // it before reaching here. We still guard with a `?` so the
    // pipeline fails fast on a stale path.
    let file_size = src_endpoint
        .lstat(src)
        .map_err(|e| anyhow!("stat source: {}", e))?
        .size;

    // Ensure destination parent exists. Ssh uses the endpoint's
    // recursive mkdir; Local uses std::fs.
    if let Some(parent) = dst.parent() {
        dst_endpoint
            .mkdir_all(parent)
            .map_err(|e| anyhow!("mkdir destination parent: {}", e))?;
    }

    // Configure the source-side hasher if verification is enabled.
    let src_hasher: Option<Box<dyn HashStrategy>> = if options.verify_after_copy {
        Some(create_hasher(options.hash_algorithm))
    } else {
        None
    };

    let dst_hasher_algo = options.hash_algorithm;
    let verify_active = options.verify_after_copy;

    // Backpressure channel between reader and writer.
    let (block_tx, mut block_rx) = mpsc::channel::<Vec<u8>>(4);

    // -----------------------------------------------------------------
    // ETAPA 1: Hilo lector
    // -----------------------------------------------------------------
    let is_paused_reader = Arc::clone(&is_paused);
    let is_cancelled_reader = Arc::clone(&is_cancelled);
    let options_reader = options.clone();
    let src_endpoint_reader = src_endpoint.clone();
    let src_reader = src.to_path_buf();

    let reader_handle =
        tokio::task::spawn_blocking(move || -> Result<Option<String>, anyhow::Error> {
            // Two readers for the Local-with-Direct case: a standard
            // one and a Direct-IO one. For the non-Direct Local case
            // (and for any Ssh case) we only open the standard reader
            // through the endpoint.
            let mut std_reader: std::fs::File = if local_direct_io {
                std::fs::File::open(super::direct_io::to_long_path(&src_reader))
                    .map_err(|e| anyhow!("opening source for fallback: {}", e))?
            } else {
                // We need *some* local std::fs::File for the non-Direct
                // path when local_direct_io is false. We open it through
                // the endpoint only when the endpoint is Local; for Ssh
                // the `direct_read_f` stays None and the `endpoint`
                // reader is used directly.
                //
                // (We can't unify the Local and Ssh branches through a
                // `Box<dyn Read>` because the Direct path needs the
                // raw `File` for `seek` calls between aligned /
                // unaligned reads.)
                if src_endpoint_reader.is_local() {
                    let path = super::direct_io::to_long_path(&src_reader);
                    std::fs::File::open(&path).map_err(|e| anyhow!("opening source: {}", e))?
                } else {
                    // Ssh with no Direct I/O: we open a Box<dyn Read>
                    // via the endpoint. We can't keep it in the
                    // `std_reader` slot, so use a dummy File and use
                    // the endpoint reader separately. This path is
                    // only hit when neither side requested Direct I/O.
                    std::fs::File::open(super::direct_io::to_long_path(&src_reader))
                        .map_err(|e| anyhow!("opening source: {}", e))?
                }
            };
            let mut direct_reader: Option<std::fs::File> = if local_direct_io {
                match super::direct_io::open_reader_direct(&src_reader, true) {
                    Ok(f) => Some(f),
                    Err(_) => None, // already fell back to standard I/O inside open_reader_direct
                }
            } else {
                None
            };
            let mut use_direct = local_direct_io && direct_reader.is_some();
            let mut aligned_buf = if use_direct {
                Some(super::direct_io::AlignedBuffer::new(buffer_bytes, 4096))
            } else {
                None
            };

            // For Ssh without Direct I/O, open the reader through the
            // endpoint. We only use it when `!use_direct`.
            let mut endpoint_reader: Option<Box<dyn std::io::Read + Send>> =
                if !local_direct_io && !src_endpoint_reader.is_local() {
                    Some(
                        src_endpoint_reader
                            .open_reader(&src_reader)
                            .map_err(|e| anyhow!("opening source: {}", e))?,
                    )
                } else {
                    None
                };

            let mut hasher = src_hasher;
            let start_time = Instant::now();
            let mut total_bytes_read = 0u64;
            let mut offset: u64 = 0;

            loop {
                if is_cancelled_reader.load(Ordering::Relaxed) {
                    return Err(anyhow!("Transfer cancelled"));
                }
                while is_paused_reader.load(Ordering::Relaxed) {
                    if is_cancelled_reader.load(Ordering::Relaxed) {
                        return Err(anyhow!("Transfer cancelled"));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }

                // Read a block. With Direct I/O we use the aligned
                // buffer; otherwise we read into a plain Vec<u8> of
                // `buffer_bytes` capacity.
                let (bytes_read, chunk) = if use_direct {
                    let buf = aligned_buf
                        .as_mut()
                        .expect("aligned_buf is Some when use_direct")
                        .as_mut_slice();
                    let remaining = file_size.saturating_sub(offset);
                    if remaining < 4096 || remaining % 4096 != 0 {
                        // Last (unaligned) block: fall back to standard I/O.
                        use_direct = false;
                        std_reader.seek(SeekFrom::Start(offset))?;
                        let n = std_reader.read(buf)?;
                        (n, buf[..n].to_vec())
                    } else {
                        let direct_f = direct_reader
                            .as_mut()
                            .expect("direct_reader is Some when use_direct");
                        match direct_f.read(buf) {
                            Ok(n) => (n, buf[..n].to_vec()),
                            Err(_) => {
                                // Direct I/O rejected this read
                                // (unaligned file system, etc.) —
                                // fall back to standard I/O for the
                                // rest of the stream.
                                use_direct = false;
                                std_reader.seek(SeekFrom::Start(offset))?;
                                let n = std_reader.read(buf)?;
                                (n, buf[..n].to_vec())
                            }
                        }
                    }
                } else if let Some(reader) = endpoint_reader.as_mut() {
                    // Ssh non-Direct: read into a fresh buffer.
                    let mut buf = vec![0u8; buffer_bytes];
                    let n = reader.read(&mut buf)?;
                    buf.truncate(n);
                    (n, buf)
                } else {
                    // Local non-Direct: read into a fresh buffer using
                    // the standard file handle.
                    let mut buf = vec![0u8; buffer_bytes];
                    let n = std_reader.read(&mut buf)?;
                    buf.truncate(n);
                    (n, buf)
                };

                if bytes_read == 0 {
                    break;
                }

                if let Some(ref mut h) = hasher {
                    h.update(&chunk);
                }

                if block_tx.blocking_send(chunk).is_err() {
                    if is_cancelled_reader.load(Ordering::Relaxed) {
                        return Err(anyhow!("Transfer cancelled by user"));
                    }
                    return Err(anyhow!(
                        "Writer thread exited before the reader finished \
                     (likely an I/O error on the destination side)"
                    ));
                }

                offset += bytes_read as u64;
                total_bytes_read += bytes_read as u64;

                if let Some(rate) = options_reader.limit_bandwidth_rate {
                    if rate > 0 {
                        let expected_duration =
                            Duration::from_secs_f64(total_bytes_read as f64 / rate as f64);
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

    // -----------------------------------------------------------------
    // ETAPA 2: Hilo escritor
    // -----------------------------------------------------------------
    let is_paused_writer = Arc::clone(&is_paused);
    let is_cancelled_writer = Arc::clone(&is_cancelled);
    let event_tx_writer = event_tx.clone();
    let _options_writer = options.clone();
    let dst_endpoint_writer = dst_endpoint.clone();
    let dst_writer = dst.to_path_buf();

    let writer_handle =
        tokio::task::spawn_blocking(move || -> Result<(u64, Option<String>), anyhow::Error> {
            let mut hasher = if verify_active {
                Some(create_hasher(dst_hasher_algo))
            } else {
                None
            };

            // Set up the writers. For Local with Direct I/O we
            // open a Direct file and a standard fallback. For the
            // non-Direct Local case we open a single standard file.
            // For Ssh (no Direct I/O ever) we use the endpoint
            // writer.
            let mut std_writer: std::fs::File = if dst_endpoint_writer.is_local() {
                std::fs::File::create(super::direct_io::to_long_path(&dst_writer))
                    .map_err(|e| anyhow!("opening destination: {}", e))?
            } else {
                // Ssh destination: we still need a local File to
                // satisfy the `std_writer` slot for the Ssh +
                // local_direct_io path (which never fires because
                // local_direct_io requires both endpoints to be
                // Local). This branch is only reached when
                // local_direct_io is false and dst is Ssh; the
                // endpoint writer is used directly.
                std::fs::File::create(super::direct_io::to_long_path(&dst_writer))
                    .map_err(|e| anyhow!("opening destination (fallback): {}", e))?
            };
            let mut direct_writer: Option<std::fs::File> = if local_direct_io {
                match super::direct_io::open_writer_direct(&dst_writer, true) {
                    Ok(f) => Some(f),
                    Err(_) => None,
                }
            } else {
                None
            };
            let mut use_direct = local_direct_io && direct_writer.is_some();
            let mut aligned_writer_buf = if use_direct {
                Some(super::direct_io::AlignedBuffer::new(buffer_bytes, 4096))
            } else {
                None
            };
            let mut endpoint_writer: Option<Box<dyn std::io::Write + Send>> =
                if !use_direct && !dst_endpoint_writer.is_local() {
                    Some(
                        dst_endpoint_writer
                            .open_writer(&dst_writer, true)
                            .map_err(|e| anyhow!("opening destination: {}", e))?,
                    )
                } else {
                    None
                };

            let mut offset: u64 = 0;
            let mut bytes_written_total = 0u64;
            let mut last_progress_sent = Instant::now();
            let progress_interval = Duration::from_millis(150);

            while let Some(chunk) = block_rx.blocking_recv() {
                if is_cancelled_writer.load(Ordering::Relaxed) {
                    return Err(anyhow!("Transfer cancelled"));
                }
                while is_paused_writer.load(Ordering::Relaxed) {
                    if is_cancelled_writer.load(Ordering::Relaxed) {
                        return Err(anyhow!("Transfer cancelled"));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }

                let chunk_len = chunk.len();

                if use_direct {
                    if chunk_len < 4096 || chunk_len % 4096 != 0 {
                        // Last (unaligned) chunk: fall back to
                        // standard I/O.
                        use_direct = false;
                        std_writer.seek(SeekFrom::Start(offset))?;
                        std_writer.write_all(&chunk)?;
                    } else if let Some(buf_slot) = aligned_writer_buf.as_mut() {
                        buf_slot.as_mut_slice()[..chunk_len].copy_from_slice(&chunk);
                        let direct_f = direct_writer
                            .as_mut()
                            .expect("direct_writer is Some when use_direct");
                        match direct_f.write_all(&buf_slot.as_slice()[..chunk_len]) {
                            Ok(_) => {}
                            Err(_) => {
                                // Direct I/O rejected this write
                                // (unaligned file system, etc.) —
                                // fall back to standard I/O.
                                use_direct = false;
                                std_writer.seek(SeekFrom::Start(offset))?;
                                std_writer.write_all(&chunk)?;
                            }
                        }
                    }
                } else if let Some(w) = endpoint_writer.as_mut() {
                    w.write_all(&chunk)?;
                } else {
                    std_writer.write_all(&chunk)?;
                }

                bytes_written_total += chunk_len as u64;
                offset += chunk_len as u64;

                if let Some(ref mut h) = hasher {
                    h.update(&chunk);
                }

                bytes_transferred_acc.fetch_add(chunk_len as u64, Ordering::SeqCst);

                if last_progress_sent.elapsed() >= progress_interval {
                    last_progress_sent = Instant::now();
                    let _ = event_tx_writer.send(TransferEvent::FileProgress {
                        job_id,
                        bytes_copied: bytes_written_total,
                        bytes_total: file_size,
                    });
                }
            }

            // Sync to disk. We only fsync when the destination is
            // Local; SFTP has no portable fsync primitive and the
            // SFTP close is the durability hook on the server side.
            if dst_endpoint_writer.is_local() {
                let _ = std_writer.sync_all();
            }

            let hash_result = hasher.map(|h| h.finalize());
            Ok((bytes_written_total, hash_result))
        });

    // Wait for both stages.
    let (reader_res, writer_res) = tokio::join!(reader_handle, writer_handle);

    let (_bytes_written, dst_hash) =
        writer_res.map_err(|e| anyhow!("Writer task join error: {}", e))??;
    let src_hash = reader_res.map_err(|e| anyhow!("Reader task join error: {}", e))??;

    Ok((src_hash, dst_hash))
}

// =========================================================================
//   Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::transfer::options::BufferSize;
    use std::io::Write;
    use std::sync::atomic::AtomicU64;
    use uuid::Uuid;

    fn ep() -> TransferEndpoint {
        TransferEndpoint::Local
    }

    fn shared_arc<T>(v: T) -> Arc<T> {
        Arc::new(v)
    }

    #[tokio::test]
    async fn copy_local_to_local_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.bin");
        let dst = tmp.path().join("b.bin");
        let payload: Vec<u8> = (0..4096 * 3).map(|i| (i % 251) as u8).collect();
        std::fs::File::create(&src)
            .unwrap()
            .write_all(&payload)
            .unwrap();

        let (tx, _rx) = mpsc::unbounded_channel();
        let is_paused = shared_arc(AtomicBool::new(false));
        let is_cancelled = shared_arc(AtomicBool::new(false));
        let bytes_acc = shared_arc(AtomicU64::new(0));
        let options = TransferOptions::default();

        let (src_hash, dst_hash) = copy_file_pipelined(
            &ep(),
            &src,
            &ep(),
            &dst,
            &options,
            &tx,
            Uuid::new_v4(),
            is_paused,
            is_cancelled,
            bytes_acc,
        )
        .await
        .expect("copy succeeds");

        // Verification is off by default so the hashes are None.
        assert!(src_hash.is_none());
        assert!(dst_hash.is_none());
        assert_eq!(std::fs::read(&dst).unwrap(), payload);
    }

    #[tokio::test]
    async fn copy_local_to_local_with_verify() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.bin");
        let dst = tmp.path().join("b.bin");
        let payload = b"verify-me-pipeline".to_vec();
        std::fs::File::create(&src)
            .unwrap()
            .write_all(&payload)
            .unwrap();

        let (tx, _rx) = mpsc::unbounded_channel();
        let is_paused = shared_arc(AtomicBool::new(false));
        let is_cancelled = shared_arc(AtomicBool::new(false));
        let bytes_acc = shared_arc(AtomicU64::new(0));
        let mut options = TransferOptions::default();
        options.verify_after_copy = true;

        let (src_hash, dst_hash) = copy_file_pipelined(
            &ep(),
            &src,
            &ep(),
            &dst,
            &options,
            &tx,
            Uuid::new_v4(),
            is_paused,
            is_cancelled,
            bytes_acc,
        )
        .await
        .expect("copy with verify succeeds");

        let s = src_hash.expect("src hash populated");
        let d = dst_hash.expect("dst hash populated");
        assert_eq!(s, d, "source and destination hash must match");
    }

    #[tokio::test]
    async fn copy_local_to_local_with_bandwidth_limit() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.bin");
        let dst = tmp.path().join("b.bin");
        // 256 KiB of zeros.
        let payload = vec![0u8; 256 * 1024];
        std::fs::File::create(&src)
            .unwrap()
            .write_all(&payload)
            .unwrap();

        let (tx, _rx) = mpsc::unbounded_channel();
        let is_paused = shared_arc(AtomicBool::new(false));
        let is_cancelled = shared_arc(AtomicBool::new(false));
        let bytes_acc = shared_arc(AtomicU64::new(0));
        let mut options = TransferOptions::default();
        // 64 KiB/s: copying 256 KiB should take at least 4 seconds.
        // We use 1 MiB/s and a small file so the test is fast
        // (256 KiB → ~0.25 s minimum).
        options.limit_bandwidth_rate = Some(1024 * 1024);
        options.buffer_size = BufferSize::_64KB;

        let start = std::time::Instant::now();
        let _ = copy_file_pipelined(
            &ep(),
            &src,
            &ep(),
            &dst,
            &options,
            &tx,
            Uuid::new_v4(),
            is_paused,
            is_cancelled,
            bytes_acc,
        )
        .await
        .expect("throttled copy succeeds");
        let elapsed = start.elapsed();

        // 256 KiB at 1 MiB/s = 0.25 s. Allow a generous lower
        // bound to avoid CI flakes; the throttling path is
        // sleep-based, not precision-based.
        assert!(
            elapsed >= std::time::Duration::from_millis(200),
            "throttle should delay the copy (got {:?})",
            elapsed
        );
        assert_eq!(std::fs::read(&dst).unwrap(), payload);
    }

    #[tokio::test]
    async fn copy_local_to_local_cancellation_aborts() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.bin");
        let dst = tmp.path().join("b.bin");
        // Make a payload big enough that the reader won't finish
        // before we flip the cancellation flag.
        let payload = vec![0u8; 4 * 1024 * 1024];
        std::fs::File::create(&src)
            .unwrap()
            .write_all(&payload)
            .unwrap();

        let (tx, _rx) = mpsc::unbounded_channel();
        let is_paused = shared_arc(AtomicBool::new(false));
        let is_cancelled = shared_arc(AtomicBool::new(true)); // pre-cancel
        let bytes_acc = shared_arc(AtomicU64::new(0));
        let options = TransferOptions::default();

        let res = copy_file_pipelined(
            &ep(),
            &src,
            &ep(),
            &dst,
            &options,
            &tx,
            Uuid::new_v4(),
            is_paused,
            is_cancelled,
            bytes_acc,
        )
        .await;

        assert!(res.is_err(), "pre-cancelled copy must error");
    }
}
