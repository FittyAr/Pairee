use anyhow::anyhow;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
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
//   Compress pipeline (A2-A4)
// =========================================================================

/// Bundle `sources` (a mix of files and directories
/// on `src_endpoint`) into a single archive at
/// `archive` (on `dst_endpoint`).
///
/// The function is endpoint-polymorphic: it can
/// compress local→local, local→SSH, SSH→local and
/// SSH→SSH. Each source file is read through
/// `src_endpoint.open_reader` and the bytes flow
/// straight into the archive writer; the writer
/// itself goes through `dst_endpoint.open_writer`.
///
/// Returns the final archive size in bytes.
///
/// Currently only the `Zip` branch is implemented
/// (A2); `TarGz` and `SevenZ` return a clear
/// "not yet implemented" error and will land in
/// A3 and A4.
#[allow(clippy::too_many_arguments)]
pub async fn compress_pipeline(
    src_endpoint: &TransferEndpoint,
    sources: Vec<std::path::PathBuf>,
    dst_endpoint: &TransferEndpoint,
    archive: &Path,
    format: super::job::ArchiveFormat,
    level: u8,
    event_tx: &mpsc::UnboundedSender<TransferEvent>,
    job_id: uuid::Uuid,
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
    bytes_transferred_acc: Arc<std::sync::atomic::AtomicU64>,
) -> Result<u64, anyhow::Error> {
    // Honour cancellation up front: a pre-cancelled
    // job should fail immediately, not start
    // allocating the writer.
    if is_cancelled.load(Ordering::Relaxed) {
        return Err(anyhow!("Compress cancelled before start"));
    }

    // The archive is a single file: the destination
    // parent directory must exist.
    if let Some(parent) = archive.parent() {
        if !parent.as_os_str().is_empty() {
            dst_endpoint
                .mkdir_all(parent)
                .map_err(|e| anyhow!("Failed to create parent dir {:?}: {}", parent, e))?;
        }
    }

    let _ = (event_tx, job_id, is_paused, bytes_transferred_acc);
    match format {
        super::job::ArchiveFormat::Zip => {
            compress_zip(
                src_endpoint,
                &sources,
                dst_endpoint,
                archive,
                level,
            )
            .await
        }
        super::job::ArchiveFormat::TarGz => {
            compress_targz(
                src_endpoint,
                &sources,
                dst_endpoint,
                archive,
                level,
            )
            .await
        }
        super::job::ArchiveFormat::SevenZ => {
            compress_sevenz(
                src_endpoint,
                &sources,
                dst_endpoint,
                archive,
                level,
            )
            .await
        }
    }
}

/// Zip implementation of [`compress_pipeline`].
/// Delegates the heavy lifting to the `zip` crate
/// (already in `Cargo.toml`, no new dependencies).
/// Source files are streamed through
/// `src_endpoint.open_reader`; the archive itself
/// is written to a `Vec<u8>` first because the
/// `zip` crate requires a seekable writer and the
/// endpoint abstraction only exposes
/// `Write + Send`. The buffer is flushed to
/// `dst_endpoint.open_writer` at the end. The
/// in-memory buffer is acceptable for the common
/// case (a few hundred MB); for truly huge
/// archives we would need a different approach
/// (e.g. a temp file on the destination).
///
/// `level == 0` maps to "store only" (no
/// compression); values 1-9 map to the standard
/// DEFLATE levels.
async fn compress_zip(
    src_endpoint: &TransferEndpoint,
    sources: &[std::path::PathBuf],
    dst_endpoint: &TransferEndpoint,
    archive: &Path,
    level: u8,
) -> Result<u64, anyhow::Error> {
    use std::io::Cursor;
    use zip::write::SimpleFileOptions;

    let buf: Vec<u8> = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = zip::ZipWriter::new(cursor);

    let compression = if level == 0 {
        zip::CompressionMethod::Stored
    } else {
        zip::CompressionMethod::Deflated
    };
    // `Stored` (level 0) doesn't accept a
    // `compression_level`; setting one triggers an
    // "unsupported compression level" error inside
    // the crate. We only pass the level when we
    // actually picked `Deflated`. The options are
    // rebuilt per file because we also stamp the
    // source's permission bits (Unix mode) so a
    // 0o600 file does not end up world-readable in
    // the archive.
    let options_for = |mode: Option<u32>| -> SimpleFileOptions {
        let mut opts = if matches!(compression, zip::CompressionMethod::Stored) {
            SimpleFileOptions::default()
                .compression_method(compression)
                .large_file(true)
        } else {
            SimpleFileOptions::default()
                .compression_method(compression)
                .compression_level(Some(level.clamp(1, 9) as i64))
                .large_file(true)
        };
        if let Some(m) = mode {
            opts = opts.unix_permissions(m & 0o7777);
        }
        opts
    };

    // Walk each source. For a directory, descend
    // recursively. For a file, write its bytes
    // into a single archive entry.
    for src in sources {
        if src_endpoint.is_dir(src) {
            let dir_name = src
                .file_name()
                .ok_or_else(|| anyhow!("Source has no file name: {:?}", src))?;
            let dir_prefix = Path::new(dir_name).to_path_buf();
            // Directories use the default 0o755; we don't
            // need a per-file mode for them.
            write_zip_dir(
                &mut zip,
                src_endpoint,
                src,
                &dir_prefix,
                options_for(None),
            )?;
        } else {
            // Single-file source.
            let entry_name = src
                .file_name()
                .ok_or_else(|| anyhow!("Source has no file name: {:?}", src))?;
            let stat = src_endpoint.lstat(src).ok();
            let file_mode = stat.as_ref().and_then(|m| m.mode);
            zip.start_file(
                entry_name.to_string_lossy(),
                options_for(file_mode),
            )
            .map_err(|e| anyhow!("zip start_file: {}", e))?;
            let mut reader = src_endpoint
                .open_reader(src)
                .map_err(|e| anyhow!("open reader for {:?}: {}", src, e))?;
            std::io::copy(&mut reader, &mut zip)
                .map_err(|e| anyhow!("zip write {:?}: {}", src, e))?;
        }
    }

    let cursor = zip
        .finish()
        .map_err(|e| anyhow!("zip finish: {}", e))?;
    let bytes = cursor.into_inner();

    // Flush the assembled archive to the
    // destination endpoint.
    let mut writer = dst_endpoint
        .open_writer(archive, /* overwrite = */ true)
        .map_err(|e| anyhow!("Failed to open archive for writing: {}", e))?;
    writer
        .write_all(&bytes)
        .map_err(|e| anyhow!("write archive bytes: {}", e))?;
    writer
        .flush()
        .map_err(|e| anyhow!("flush archive: {}", e))?;
    drop(writer);

    Ok(bytes.len() as u64)
}

/// 7Z implementation of [`compress_pipeline`]. Wraps
/// the source endpoint's readers into
/// `sevenz_rust::SevenZWriter` over an in-memory
/// `Vec<u8>`, then flushes the assembled archive to
/// the destination endpoint.
///
/// The `sevenz-rust` API is synchronous, so we run
/// the actual encoding inside
/// `tokio::task::spawn_blocking` to keep the runtime
/// responsive. The in-memory buffer has the same
/// caveat as Zip: for huge archives (gigabytes) we'd
/// need a temp-file path on the destination, but the
/// common case is well under 1 GiB.
///
/// `level` is forwarded as the LZMA compression
/// level (0-9). 0 maps to the crate default; the
/// 7Z format itself always compresses.
async fn compress_sevenz(
    src_endpoint: &TransferEndpoint,
    sources: &[std::path::PathBuf],
    dst_endpoint: &TransferEndpoint,
    archive: &Path,
    level: u8,
) -> Result<u64, anyhow::Error> {
    // The sevenz-rust crate is sync, so the encoding
    // work goes through spawn_blocking. We pass
    // everything it needs by clone (paths are
    // owned, endpoints are Clone).
    let src_endpoint = src_endpoint.clone();
    let sources = sources.to_vec();
    let dst_endpoint = dst_endpoint.clone();
    let archive = archive.to_path_buf();
    let level_u32 = level.clamp(0, 9) as u32;

    let bytes = tokio::task::spawn_blocking(move || {
        use sevenz_rust::{SevenZArchiveEntry, SevenZWriter};
        use std::io::Cursor;

        let buf: Vec<u8> = Vec::new();
        let cursor = Cursor::new(buf);
        let mut writer = SevenZWriter::new(cursor)
            .map_err(|e| anyhow!("sevenz writer: {}", e))?;
        // Configure the LZMA2 level globally on the
        // writer. The sevenz-rust API takes a
        // `MethodOptions::Num(level)` for the LZMA
        // level; the engine's `level` is the LZMA
        // level (0-9). `level == 0` keeps the crate
        // default.
        //
        // TODO: `sevenz-rust 0.6.1` panics on
        // `attempt to multiply with overflow` when
        // the level is set via set_content_methods
        // (its encoder does `1u32 << (level + 11)`
        // for dict size, and the math goes wrong
        // somewhere). For now we always use the
        // crate default and accept the user's level
        // as a no-op. The engine's `level` is still
        // captured so the UI can show it; the bytes
        // are encoded at whatever the crate thinks
        // is the default.
        let _ = level_u32;

        for src in &sources {
            if src_endpoint.is_dir(src) {
                let dir_name = src
                    .file_name()
                    .ok_or_else(|| anyhow!("Source has no file name: {:?}", src))?;
                push_sevenz_dir(
                    &mut writer,
                    &src_endpoint,
                    src,
                    dir_name.to_string_lossy().as_ref(),
                )?;
            } else {
                let entry_name = src
                    .file_name()
                    .ok_or_else(|| anyhow!("Source has no file name: {:?}", src))?
                    .to_string_lossy()
                    .into_owned();
                let mut reader = src_endpoint
                    .open_reader(src)
                    .map_err(|e| anyhow!("open reader for {:?}: {}", src, e))?;
                let entry = SevenZArchiveEntry::from_path(src, entry_name);
                writer
                    .push_archive_entry(entry, Some(&mut reader))
                    .map_err(|e| anyhow!("sevenz push_entry: {}", e))?;
            }
        }

        let cursor = writer
            .finish()
            .map_err(|e| anyhow!("sevenz finish: {}", e))?;
        let bytes = cursor.into_inner();
        Ok::<Vec<u8>, anyhow::Error>(bytes)
    })
    .await
    .map_err(|e| anyhow!("spawn_blocking join: {}", e))??;

    // Flush the assembled archive to the destination.
    let mut writer = dst_endpoint
        .open_writer(&archive, /* overwrite = */ true)
        .map_err(|e| anyhow!("Failed to open archive for writing: {}", e))?;
    std::io::Write::write_all(&mut writer, &bytes)
        .map_err(|e| anyhow!("write archive bytes: {}", e))?;
    std::io::Write::flush(&mut writer)
        .map_err(|e| anyhow!("flush archive: {}", e))?;
    drop(writer);

    Ok(bytes.len() as u64)
}

// =========================================================================
//   Extract pipeline (A7-A9)
// =========================================================================

/// Unpack a single archive at `archive` (on
/// `src_endpoint`) into `dst_dir` (on
/// `dst_endpoint`).
///
/// The `format` is required: the engine does not
/// auto-detect from the file extension because the
/// caller (the UI popup) already knows the format
/// from the user's selection.
///
/// Path-traversal entries (`..`, absolute paths,
/// NUL bytes) are rejected. We reuse the same
/// `validate_archive_entry_name` helper that the
/// legacy `fs/archive.rs` already had, so the
/// behaviour matches.
#[allow(clippy::too_many_arguments)]
pub async fn extract_pipeline(
    src_endpoint: &TransferEndpoint,
    archive: &Path,
    dst_endpoint: &TransferEndpoint,
    dst_dir: &Path,
    format: super::job::ArchiveFormat,
    event_tx: &mpsc::UnboundedSender<TransferEvent>,
    job_id: uuid::Uuid,
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
) -> Result<u64, anyhow::Error> {
    if is_cancelled.load(Ordering::Relaxed) {
        return Err(anyhow!("Extract cancelled before start"));
    }

    // Make sure the destination directory exists
    // (it almost always does, but the user might
    // have typed a fresh one).
    dst_endpoint
        .mkdir_all(dst_dir)
        .map_err(|e| anyhow!("Failed to create dst dir {:?}: {}", dst_dir, e))?;

    let _ = (event_tx, job_id, is_paused);
    match format {
        super::job::ArchiveFormat::Zip => {
            extract_zip(src_endpoint, archive, dst_endpoint, dst_dir).await
        }
        super::job::ArchiveFormat::TarGz => {
            extract_targz(src_endpoint, archive, dst_endpoint, dst_dir).await
        }
        super::job::ArchiveFormat::SevenZ => {
            extract_sevenz(src_endpoint, archive, dst_endpoint, dst_dir).await
        }
    }
}

/// Reject names that try to escape the destination
/// directory. Returns the sanitised name (forward
/// slashes only) on success.
fn sanitise_entry_name(raw: &str) -> Result<PathBuf, anyhow::Error> {
    if raw.is_empty() {
        return Err(anyhow!("archive entry has empty name"));
    }
    if raw.contains('\0') {
        return Err(anyhow!("archive entry contains NUL byte"));
    }
    // No absolute paths.
    if raw.starts_with('/') || raw.starts_with('\\') {
        return Err(anyhow!("archive entry is absolute: {}", raw));
    }
    // No parent traversal.
    let normalised = raw.replace('\\', "/");
    for component in std::path::Path::new(&normalised).components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(anyhow!("archive entry has '..': {}", raw));
            }
            std::path::Component::RootDir => {
                return Err(anyhow!("archive entry is rooted: {}", raw));
            }
            // Reject Windows drive-relative and drive-absolute
            // paths (e.g. `C:foo`, `C:/foo`, `C:\foo`,
            // `\\?\C:\foo`, `\\server\share\foo`).
            //
            // Without this check, a name like `C:/Users/victim/
            // .ssh/id_rsa` passes the leading-/\\ check on
            // Windows (it does not start with `/` or `\\`),
            // the component walker only rejects `ParentDir`
            // and `RootDir`, and then `dst_dir.join("C:/...")
            // ` is treated as **absolute** by `Path::join`,
            // which replaces the base. The file would land
            // on `C:\Users\victim\.ssh\id_rsa` even though
            // the user thought they were extracting under
            // their chosen destination.
            std::path::Component::Prefix(_) => {
                return Err(anyhow!(
                    "archive entry has drive / device prefix: {}",
                    raw
                ));
            }
            _ => {}
        }
    }
    // Belt and suspenders: on Windows, even a path the
    // component walker accepted could be absolute (e.g.
    // very long UNC-style names that the walker only sees
    // partially). `is_absolute()` on the joined path is
    // cheap, so reject anything that smells absolute.
    if std::path::Path::new(&normalised).is_absolute() {
        return Err(anyhow!("archive entry resolves to absolute: {}", raw));
    }
    Ok(PathBuf::from(normalised))
}

async fn extract_zip(
    src_endpoint: &TransferEndpoint,
    archive: &Path,
    dst_endpoint: &TransferEndpoint,
    dst_dir: &Path,
) -> Result<u64, anyhow::Error> {
    use std::io::{Cursor, Read, Write};

    // For Local, we can hand the path straight to
    // `zip::ZipArchive::new`. For Ssh we have to
    // slurp the file into memory first because the
    // zip crate takes a `Read + Seek` and the Ssh
    // reader is `Read` only.
    let mut archive_buf: Vec<u8> = Vec::new();
    if src_endpoint.is_local() {
        let mut f = std::fs::File::open(archive)
            .map_err(|e| anyhow!("open archive {:?}: {}", archive, e))?;
        f.read_to_end(&mut archive_buf)
            .map_err(|e| anyhow!("read archive: {}", e))?;
    } else {
        let mut reader = src_endpoint
            .open_reader(archive)
            .map_err(|e| anyhow!("open archive reader: {}", e))?;
        reader
            .read_to_end(&mut archive_buf)
            .map_err(|e| anyhow!("read archive bytes: {}", e))?;
    }

    let cursor = Cursor::new(archive_buf);
    let mut za = zip::ZipArchive::new(cursor)
        .map_err(|e| anyhow!("zip open: {}", e))?;

    let mut entries_written: u64 = 0;
    for i in 0..za.len() {
        let mut entry = za
            .by_index(i)
            .map_err(|e| anyhow!("zip entry {}: {}", i, e))?;
        if entry.is_dir() {
            // Directories are implicit: we create
            // the parent when writing the file
            // contents, so we skip explicit dir
            // entries.
            continue;
        }
        let raw_name = entry.name().to_string();
        let name = sanitise_entry_name(&raw_name)
            .map_err(|e| anyhow!("unsafe entry '{}': {}", raw_name, e))?;
        let out_path = dst_dir.join(&name);
        if let Some(parent) = out_path.parent() {
            dst_endpoint
                .mkdir_all(parent)
                .map_err(|e| anyhow!("mkdir_all {:?}: {}", parent, e))?;
        }
        let mut writer = dst_endpoint
            .open_writer(&out_path, /* overwrite = */ true)
            .map_err(|e| anyhow!("open writer for {:?}: {}", out_path, e))?;
        // Stream the entry bytes through the
        // endpoint writer.
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = entry
                .read(&mut buf)
                .map_err(|e| anyhow!("zip read entry: {}", e))?;
            if n == 0 {
                break;
            }
            writer
                .write_all(&buf[..n])
                .map_err(|e| anyhow!("write entry bytes: {}", e))?;
        }
        writer
            .flush()
            .map_err(|e| anyhow!("flush entry: {}", e))?;
        drop(writer);
        entries_written += 1;
    }

    Ok(entries_written)
}

async fn extract_targz(
    src_endpoint: &TransferEndpoint,
    archive: &Path,
    dst_endpoint: &TransferEndpoint,
    dst_dir: &Path,
) -> Result<u64, anyhow::Error> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    // Slurp the .tar.gz into memory (same caveat
    // as Zip: tar's Archive needs Read + Seek and
    // the Ssh reader is Read only).
    let mut bytes: Vec<u8> = Vec::new();
    if src_endpoint.is_local() {
        let mut f = std::fs::File::open(archive)
            .map_err(|e| anyhow!("open archive {:?}: {}", archive, e))?;
        f.read_to_end(&mut bytes)
            .map_err(|e| anyhow!("read archive: {}", e))?;
    } else {
        let mut reader = src_endpoint
            .open_reader(archive)
            .map_err(|e| anyhow!("open archive reader: {}", e))?;
        reader
            .read_to_end(&mut bytes)
            .map_err(|e| anyhow!("read archive bytes: {}", e))?;
    }

    let gz = GzDecoder::new(&bytes[..]);
    let mut tar = tar::Archive::new(gz);
    let mut entries_written: u64 = 0;
    for entry in tar
        .entries()
        .map_err(|e| anyhow!("tar entries: {}", e))?
    {
        let mut entry = entry.map_err(|e| anyhow!("tar entry: {}", e))?;
        let header = entry
            .path()
            .map_err(|e| anyhow!("tar entry path: {}", e))?
            .to_path_buf();
        let name_str = header
            .to_str()
            .ok_or_else(|| anyhow!("non-UTF8 path in tar entry"))?;
        let name = sanitise_entry_name(name_str)
            .map_err(|e| anyhow!("unsafe tar entry '{}': {}", name_str, e))?;
        let out_path = dst_dir.join(&name);

        if entry.header().entry_type().is_dir() {
            dst_endpoint
                .mkdir_all(&out_path)
                .map_err(|e| anyhow!("mkdir {:?}: {}", out_path, e))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            dst_endpoint
                .mkdir_all(parent)
                .map_err(|e| anyhow!("mkdir_all {:?}: {}", parent, e))?;
        }
        let mut writer = dst_endpoint
            .open_writer(&out_path, /* overwrite = */ true)
            .map_err(|e| anyhow!("open writer for {:?}: {}", out_path, e))?;
        std::io::copy(&mut entry, &mut writer)
            .map_err(|e| anyhow!("write tar entry: {}", e))?;
        std::io::Write::flush(&mut writer)
            .map_err(|e| anyhow!("flush tar entry: {}", e))?;
        drop(writer);
        entries_written += 1;
    }

    Ok(entries_written)
}

async fn extract_sevenz(
    src_endpoint: &TransferEndpoint,
    archive: &Path,
    dst_endpoint: &TransferEndpoint,
    dst_dir: &Path,
) -> Result<u64, anyhow::Error> {
    use std::io::Read;

    // Slurp the archive into memory because
    // sevenz-rust's decompression APIs want
    // `Read + Seek` and the Ssh reader is `Read`
    // only. Same caveat as Zip and TarGz.
    let mut bytes: Vec<u8> = Vec::new();
    if src_endpoint.is_local() {
        let mut f = std::fs::File::open(archive)
            .map_err(|e| anyhow!("open archive {:?}: {}", archive, e))?;
        f.read_to_end(&mut bytes)
            .map_err(|e| anyhow!("read archive: {}", e))?;
    } else {
        let mut reader = src_endpoint
            .open_reader(archive)
            .map_err(|e| anyhow!("open archive reader: {}", e))?;
        reader
            .read_to_end(&mut bytes)
            .map_err(|e| anyhow!("read archive bytes: {}", e))?;
    }

    // sevenz-rust is sync, so the work goes
    // through spawn_blocking. The callback gets
    // the entry name, a per-entry reader, and
    // the destination path the library wants
    // to use. We override it and route through
    // our endpoint instead.
    let dst_dir = dst_dir.to_path_buf();
    let dst_endpoint_clone = dst_endpoint.clone();
    let entries_written = tokio::task::spawn_blocking(move || {
        use std::io::Cursor;
        let cursor = Cursor::new(bytes);
        let mut count: u64 = 0;
        let result: Result<(), sevenz_rust::Error> = (|| {
            sevenz_rust::decompress_with_extract_fn(
                cursor,
                &dst_dir,
                |entry, reader, _dest| {
                    let raw_name = entry.name().to_string();
                    let name = sanitise_entry_name(&raw_name).map_err(|e| {
                        sevenz_rust::Error::io_msg(
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                format!("{}: {}", e, raw_name),
                            ),
                            "",
                        )
                    })?;
                    let out_path = dst_dir.join(&name);

                    if entry.is_directory() {
                        dst_endpoint_clone.mkdir_all(&out_path).map_err(|e| {
                            sevenz_rust::Error::io_msg(
                                std::io::Error::other(format!("mkdir {:?}: {}", out_path, e)),
                                "",
                            )
                        })?;
                        return Ok(true);
                    }

                    if let Some(parent) = out_path.parent() {
                        dst_endpoint_clone.mkdir_all(parent).map_err(|e| {
                            sevenz_rust::Error::io_msg(
                                std::io::Error::other(format!("mkdir_all {:?}: {}", parent, e)),
                                "",
                            )
                        })?;
                    }
                    let mut writer = dst_endpoint_clone
                        .open_writer(&out_path, /* overwrite = */ true)
                        .map_err(|e| {
                            sevenz_rust::Error::io_msg(
                                std::io::Error::other(format!("open writer {:?}: {}", out_path, e)),
                                "",
                            )
                        })?;
                    let mut reader = reader;
                    std::io::copy(&mut reader, &mut writer).map_err(|e| {
                        sevenz_rust::Error::io_msg(
                            std::io::Error::other(format!("write 7z entry: {}", e)),
                            "",
                        )
                    })?;
                    std::io::Write::flush(&mut writer).map_err(|e| {
                        sevenz_rust::Error::io_msg(
                            std::io::Error::other(format!("flush 7z entry: {}", e)),
                            "",
                        )
                    })?;
                    drop(writer);
                    count += 1;
                    Ok(true)
                },
            )
        })();
        result.map_err(|e| anyhow!("sevenz extract: {}", e))?;
        Ok::<u64, anyhow::Error>(count)
    })
    .await
    .map_err(|e| anyhow!("spawn_blocking join: {}", e))??;

    Ok(entries_written)
}

/// Recursive helper for `compress_sevenz`: push a
/// directory tree into the writer. `name` is the
/// top-level entry name (the directory's basename).
fn push_sevenz_dir<W: std::io::Write + std::io::Seek>(
    writer: &mut sevenz_rust::SevenZWriter<W>,
    endpoint: &TransferEndpoint,
    dir: &Path,
    name: &str,
) -> Result<(), anyhow::Error> {
    let entries = endpoint
        .read_dir(dir)
        .map_err(|e| anyhow!("read_dir {:?}: {}", dir, e))?;
    for entry in entries {
        let entry_path = entry.path;
        let entry_name = entry_path
            .file_name()
            .ok_or_else(|| anyhow!("entry has no name: {:?}", entry_path))?;
        let child_name = format!("{}/{}", name, entry_name.to_string_lossy());
        if entry.is_dir {
            push_sevenz_dir(writer, endpoint, &entry_path, &child_name)?;
        } else {
            let mut reader = endpoint
                .open_reader(&entry_path)
                .map_err(|e| anyhow!("open reader for {:?}: {}", entry_path, e))?;
            let archive_entry =
                sevenz_rust::SevenZArchiveEntry::from_path(&entry_path, child_name.clone());
            writer
                .push_archive_entry(archive_entry, Some(&mut reader))
                .map_err(|e| anyhow!("sevenz push_entry: {}", e))?;
        }
    }
    Ok(())
}

/// TarGz implementation of [`compress_pipeline`].
/// Pipes a `flate2::GzEncoder` into a `tar::Builder`,
/// then through the source endpoint for each input.
/// The tar builder writes entry headers followed by
/// the file bytes; the gzip layer compresses the
/// whole stream.
///
/// `level` is forwarded to the gzip layer
/// (1-9 typical). `level == 0` falls back to
/// `flate2::Compression::default()` (currently
/// level 6) — the engine has already documented
/// that tar/7z treat 0 as "no special meaning" and
/// gzip always compresses, so we don't add a
/// "store-only" path for it.
async fn compress_targz(
    src_endpoint: &TransferEndpoint,
    sources: &[std::path::PathBuf],
    dst_endpoint: &TransferEndpoint,
    archive: &Path,
    level: u8,
) -> Result<u64, anyhow::Error> {
    use flate2::Compression;
    use flate2::write::GzEncoder;

    let buf: Vec<u8> = Vec::new();
    let compression = if level == 0 {
        Compression::default()
    } else {
        Compression::new(level.clamp(1, 9) as u32)
    };
    let encoder = GzEncoder::new(buf, compression);
    let mut builder = tar::Builder::new(encoder);

    for src in sources {
        if src_endpoint.is_dir(src) {
            let dir_name = src
                .file_name()
                .ok_or_else(|| anyhow!("Source has no file name: {:?}", src))?;
            append_tar_dir(
                &mut builder,
                src_endpoint,
                src,
                dir_name,
            )?;
        } else {
            // Single-file source. Entry name is
            // the basename. We use the low-level
            // `append` API because `append_file`
            // only accepts `&mut std::fs::File`,
            // not our endpoint-agnostic reader.
            let entry_name = src
                .file_name()
                .ok_or_else(|| anyhow!("Source has no file name: {:?}", src))?;
            let stat = src_endpoint.lstat(src).ok();
            let size = stat.as_ref().map(|m| m.size).unwrap_or(0);
            // Preserve the source's permission bits so a
            // 0o600 SSH key does not end up world-readable
            // in the archive. Fall back to 0o644 only when
            // the endpoint cannot surface a mode (e.g.
            // Windows or an SFTP server that doesn't
            // expose perm bits).
            let mode = stat
                .as_ref()
                .and_then(|m| m.mode)
                .map(|m| m & 0o7777)
                .unwrap_or(0o644);
            let mut header = tar::Header::new_gnu();
            header
                .set_path(entry_name)
                .map_err(|e| anyhow!("tar header path: {}", e))?;
            header.set_size(size);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_mode(mode);
            header.set_cksum();
            let mut reader = src_endpoint
                .open_reader(src)
                .map_err(|e| anyhow!("open reader for {:?}: {}", src, e))?;
            builder
                .append(&header, &mut reader)
                .map_err(|e| anyhow!("tar append {:?}: {}", src, e))?;
        }
    }

    let encoder = builder
        .into_inner()
        .map_err(|e| anyhow!("tar finish: {}", e))?;
    let bytes = encoder
        .finish()
        .map_err(|e| anyhow!("gz finish: {}", e))?;

    let mut writer = dst_endpoint
        .open_writer(archive, /* overwrite = */ true)
        .map_err(|e| anyhow!("Failed to open archive for writing: {}", e))?;
    std::io::Write::write_all(&mut writer, &bytes)
        .map_err(|e| anyhow!("write archive bytes: {}", e))?;
    std::io::Write::flush(&mut writer)
        .map_err(|e| anyhow!("flush archive: {}", e))?;
    drop(writer);

    Ok(bytes.len() as u64)
}

/// Recursive helper for `compress_targz`: append a
/// directory tree to the tar builder. `name` is the
/// top-level entry name (the directory's basename).
/// Sub-entries are appended with `<name>/<sub_name>`.
fn append_tar_dir<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    endpoint: &TransferEndpoint,
    dir: &Path,
    name: &std::ffi::OsStr,
) -> Result<(), anyhow::Error> {
    let entries = endpoint
        .read_dir(dir)
        .map_err(|e| anyhow!("read_dir {:?}: {}", dir, e))?;
    let mut header = tar::Header::new_gnu();
    header
        .set_path(name)
        .map_err(|e| anyhow!("tar header path: {}", e))?;
    header.set_entry_type(tar::EntryType::Directory);
    header.set_size(0);
    header.set_cksum();
    builder
        .append(&header, std::io::empty())
        .map_err(|e| anyhow!("tar append dir header: {}", e))?;

    for entry in entries {
        let entry_path = entry.path;
        let entry_name = entry_path
            .file_name()
            .ok_or_else(|| anyhow!("entry has no name: {:?}", entry_path))?;
        let child_name = {
            let mut s = name.to_os_string();
            s.push("/");
            s.push(entry_name);
            s
        };
        if entry.is_dir {
            append_tar_dir(builder, endpoint, &entry_path, &child_name)?;
        } else {
            let stat = endpoint.lstat(&entry_path).ok();
            let size = stat.as_ref().map(|m| m.size).unwrap_or(0);
            // Preserve the source's permission bits; see
            // the single-file branch above for the
            // rationale. Fall back to 0o644 when the
            // endpoint cannot surface a mode.
            let mode = stat
                .as_ref()
                .and_then(|m| m.mode)
                .map(|m| m & 0o7777)
                .unwrap_or(0o644);
            let mut header = tar::Header::new_gnu();
            header
                .set_path(&child_name)
                .map_err(|e| anyhow!("tar header path: {}", e))?;
            header.set_size(size);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_mode(mode);
            header.set_cksum();
            let mut reader = endpoint
                .open_reader(&entry_path)
                .map_err(|e| anyhow!("open reader for {:?}: {}", entry_path, e))?;
            builder
                .append(&header, &mut reader)
                .map_err(|e| anyhow!("tar append {:?}: {}", entry_path, e))?;
        }
    }
    Ok(())
}

/// Recursive helper for `compress_zip`: write a
/// directory tree into the archive. `prefix` is
/// the path already accumulated in the archive
/// entry names (e.g. `my_folder` or
/// `my_folder/sub`); the function appends each
/// entry's file name to it.
fn write_zip_dir<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    endpoint: &TransferEndpoint,
    dir: &Path,
    prefix: &Path,
    base_options: zip::write::SimpleFileOptions,
) -> Result<(), anyhow::Error> {
    let entries = endpoint
        .read_dir(dir)
        .map_err(|e| anyhow!("read_dir {:?}: {}", dir, e))?;
    for entry in entries {
        let entry_path = entry.path;
        let entry_name = entry_path
            .file_name()
            .ok_or_else(|| anyhow!("entry has no name: {:?}", entry_path))?;
        let archive_path = prefix.join(entry_name);
        if entry.is_dir {
            write_zip_dir(zip, endpoint, &entry_path, &archive_path, base_options)?;
        } else {
            let stat = endpoint.lstat(&entry_path).ok();
            let mode = stat.as_ref().and_then(|m| m.mode);
            let mut file_options = base_options;
            if let Some(m) = mode {
                file_options = file_options.unix_permissions(m & 0o7777);
            }
            zip.start_file(
                archive_path.to_string_lossy().replace('\\', "/"),
                file_options,
            )
            .map_err(|e| anyhow!("zip start_file: {}", e))?;
            let mut reader = endpoint
                .open_reader(&entry_path)
                .map_err(|e| anyhow!("open reader for {:?}: {}", entry_path, e))?;
            std::io::copy(&mut reader, zip)
                .map_err(|e| anyhow!("zip write {:?}: {}", entry_path, e))?;
        }
    }
    Ok(())
}

// =========================================================================
//   Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::transfer::options::BufferSize;
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

    // ===============================================================
    //   Compress pipeline (A2)
    // ===============================================================

    fn shared_arc_bool(b: bool) -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(b))
    }

    #[tokio::test]
    async fn compress_local_to_local_zip_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("input");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("a.txt"), b"hello-a").unwrap();
        std::fs::write(src_dir.join("b.txt"), b"hello-b").unwrap();

        let archive = tmp.path().join("out.zip");
        let (tx, _rx) = mpsc::unbounded_channel();
        let size = compress_pipeline(
            &ep(),
            vec![src_dir.clone()],
            &ep(),
            &archive,
            super::super::job::ArchiveFormat::Zip,
            6,
            &tx,
            Uuid::new_v4(),
            shared_arc_bool(false),
            shared_arc_bool(false),
            Arc::new(AtomicU64::new(0)),
        )
        .await
        .expect("zip compress succeeds");
        assert!(size > 0, "archive should not be empty");
        assert!(archive.exists(), "archive file was not written");

        // Open the archive and confirm both files are
        // present with the right content. We use a
        // fresh `zip::ZipArchive` to validate the
        // bytes.
        let f = std::fs::File::open(&archive).unwrap();
        let mut za = zip::ZipArchive::new(f).unwrap();
        let mut a = za.by_name("input/a.txt").expect("a.txt present");
        let mut buf = String::new();
        use std::io::Read;
        a.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "hello-a");
    }

    /// Regression test for security review finding L2:
    /// ZIP and tar.gz compress pipelines used to hard-code
    /// mode 0o644 on every entry, which leaks a 0o600
    /// source (e.g. an SSH key) as world-readable in the
    /// archive. The fix reads the source's `lstat().mode`
    /// and stamps it onto the entry.
    #[cfg(unix)]
    #[tokio::test]
    async fn compress_zip_preserves_source_mode_bits() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("private.bin");
        std::fs::write(&src, b"secret").unwrap();
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600)).unwrap();

        let archive = tmp.path().join("out.zip");
        let (tx, _rx) = mpsc::unbounded_channel();
        compress_pipeline(
            &ep(),
            vec![src.clone()],
            &ep(),
            &archive,
            super::super::job::ArchiveFormat::Zip,
            6,
            &tx,
            Uuid::new_v4(),
            shared_arc_bool(false),
            shared_arc_bool(false),
            Arc::new(AtomicU64::new(0)),
        )
        .await
        .expect("zip compress");

        let f = std::fs::File::open(&archive).unwrap();
        let mut za = zip::ZipArchive::new(f).unwrap();
        let entry = za.by_name("private.bin").expect("entry present");
        // The zip crate exposes the external attrs; the
        // unix perm bits are stored in the high 16 bits
        // of the external file attributes field. The
        // public API gives us `unix_mode()` directly.
        #[cfg(unix)]
        let stored_mode = entry.unix_mode().unwrap_or(0o644);
        assert_eq!(
            stored_mode & 0o777,
            0o600,
            "zip entry must preserve 0o600 source mode, got {:o}",
            stored_mode
        );
    }

    /// Same as the zip version above, but for tar.gz: the
    /// source's mode bits must end up in the tar header.
    #[cfg(unix)]
    #[tokio::test]
    async fn compress_targz_preserves_source_mode_bits() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("private.bin");
        std::fs::write(&src, b"secret").unwrap();
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o600)).unwrap();

        let archive = tmp.path().join("out.tar.gz");
        let (tx, _rx) = mpsc::unbounded_channel();
        compress_pipeline(
            &ep(),
            vec![src.clone()],
            &ep(),
            &archive,
            super::super::job::ArchiveFormat::TarGz,
            6,
            &tx,
            Uuid::new_v4(),
            shared_arc_bool(false),
            shared_arc_bool(false),
            Arc::new(AtomicU64::new(0)),
        )
        .await
        .expect("targz compress");

        // Open the archive and read the header for
        // private.bin. The `tar` crate's `Header::mode()`
        // returns the unix mode bits we stamped.
        let bytes = std::fs::read(&archive).unwrap();
        let gz = flate2::read::GzDecoder::new(&bytes[..]);
        let mut archive = tar::Archive::new(gz);
        let mut found = false;
        for entry in archive.entries().unwrap() {
            let mut e = entry.unwrap();
            let path = e.path().unwrap().to_path_buf();
            if path.file_name().and_then(|n| n.to_str()) == Some("private.bin") {
                assert_eq!(
                    e.header().mode().unwrap() & 0o777,
                    0o600,
                    "tar entry must preserve 0o600 source mode"
                );
                found = true;
            }
        }
        assert!(found, "private.bin should be present in the archive");
    }

    #[tokio::test]
    async fn compress_local_to_local_zip_zero_level_stores() {
        // With level=0 the archive should be at least
        // as large as the sum of source bytes
        // (no DEFLATE overhead). We use a deterministic
        // payload so the test is not flaky.
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("payload.bin");
        let payload = vec![0u8; 4096];
        std::fs::write(&src, &payload).unwrap();

        let archive = tmp.path().join("store.zip");
        let (tx, _rx) = mpsc::unbounded_channel();
        let size = compress_pipeline(
            &ep(),
            vec![src],
            &ep(),
            &archive,
            super::super::job::ArchiveFormat::Zip,
            0,
            &tx,
            Uuid::new_v4(),
            shared_arc_bool(false),
            shared_arc_bool(false),
            Arc::new(AtomicU64::new(0)),
        )
        .await
        .expect("zip compress (level=0) succeeds");
        // The archive adds a tiny header per entry,
        // but with no compression it must be at least
        // the size of the payload.
        assert!(size as usize >= payload.len());
    }

    #[tokio::test]
    async fn compress_local_to_local_zip_cancellation_aborts() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.txt");
        std::fs::write(&src, b"hello").unwrap();
        let archive = tmp.path().join("out.zip");
        let (tx, _rx) = mpsc::unbounded_channel();
        let res = compress_pipeline(
            &ep(),
            vec![src],
            &ep(),
            &archive,
            super::super::job::ArchiveFormat::Zip,
            6,
            &tx,
            Uuid::new_v4(),
            shared_arc_bool(false),
            shared_arc_bool(true), // pre-cancelled
            Arc::new(AtomicU64::new(0)),
        )
        .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn compress_local_to_local_sevenz_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("input");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("a.txt"), b"hello-a").unwrap();
        std::fs::write(src_dir.join("b.txt"), b"hello-b").unwrap();

        let archive = tmp.path().join("out.7z");
        let (tx, _rx) = mpsc::unbounded_channel();
        let size = compress_pipeline(
            &ep(),
            vec![src_dir.clone()],
            &ep(),
            &archive,
            super::super::job::ArchiveFormat::SevenZ,
            5,
            &tx,
            Uuid::new_v4(),
            shared_arc_bool(false),
            shared_arc_bool(false),
            Arc::new(AtomicU64::new(0)),
        )
        .await
        .expect("sevenz compress succeeds");
        assert!(size > 0);
        assert!(archive.exists());

        // Decode the archive back with the same
        // crate to confirm both files survive. We
        // use `decompress_file_with_extract_fn`
        // (the lowest-level API) because the
        // high-level `decompress_file` requires a
        // single `out_dir` and would clobber our
        // input.
        let out_dir = tmp.path().join("out");
        std::fs::create_dir_all(&out_dir).unwrap();
        sevenz_rust::decompress_file_with_extract_fn(
            &archive,
            &out_dir,
            |entry, reader, _dest| {
                let mut buf = Vec::new();
                use std::io::Read;
                let _ = reader.read_to_end(&mut buf);
                let name = entry.name().to_string();
                if name.ends_with("a.txt") {
                    assert_eq!(buf, b"hello-a");
                } else if name.ends_with("b.txt") {
                    assert_eq!(buf, b"hello-b");
                }
                Ok::<bool, sevenz_rust::Error>(true)
            },
        )
        .expect("decode succeeds");
    }

    #[tokio::test]
    async fn compress_local_to_local_targz_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("input");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("a.txt"), b"hello-a").unwrap();
        std::fs::write(src_dir.join("b.txt"), b"hello-b").unwrap();

        let archive = tmp.path().join("out.tar.gz");
        let (tx, _rx) = mpsc::unbounded_channel();
        let size = compress_pipeline(
            &ep(),
            vec![src_dir.clone()],
            &ep(),
            &archive,
            super::super::job::ArchiveFormat::TarGz,
            6,
            &tx,
            Uuid::new_v4(),
            shared_arc_bool(false),
            shared_arc_bool(false),
            Arc::new(AtomicU64::new(0)),
        )
        .await
        .expect("targz compress succeeds");
        assert!(size > 0);
        // Confirm the bytes actually decode back
        // into the same files. We use flate2 +
        // tar directly to keep the test self-
        // contained.
        let f = std::fs::File::open(&archive).unwrap();
        let gz = flate2::read::GzDecoder::new(f);
        let mut archive = tar::Archive::new(gz);
        let mut found_a = false;
        let mut found_b = false;
        for entry in archive.entries().unwrap() {
            let mut e = entry.unwrap();
            let path = e.path().unwrap().to_string_lossy().to_string();
            let mut buf = String::new();
            use std::io::Read;
            e.read_to_string(&mut buf).unwrap();
            if path == "input/a.txt" {
                assert_eq!(buf, "hello-a");
                found_a = true;
            } else if path == "input/b.txt" {
                assert_eq!(buf, "hello-b");
                found_b = true;
            }
        }
        assert!(found_a && found_b, "missing one or more files");
    }

    // ===============================================================
    //   Extract pipeline (A7-A9)
    // ===============================================================

    #[tokio::test]
    async fn extract_zip_to_local_round_trip() {
        // Build a ZIP on disk, extract it, and
        // confirm both files are present and
        // contain the expected bytes.
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("input");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("a.txt"), b"hello-a").unwrap();
        std::fs::write(src_dir.join("b.txt"), b"hello-b").unwrap();

        let archive = tmp.path().join("out.zip");
        let (tx, _rx) = mpsc::unbounded_channel();
        compress_pipeline(
            &ep(),
            vec![src_dir.clone()],
            &ep(),
            &archive,
            super::super::job::ArchiveFormat::Zip,
            6,
            &tx,
            Uuid::new_v4(),
            shared_arc_bool(false),
            shared_arc_bool(false),
            Arc::new(AtomicU64::new(0)),
        )
        .await
        .expect("zip compress succeeds");

        let dst = tmp.path().join("out");
        std::fs::create_dir_all(&dst).unwrap();
        let entries = extract_pipeline(
            &ep(),
            &archive,
            &ep(),
            &dst,
            super::super::job::ArchiveFormat::Zip,
            &tx,
            Uuid::new_v4(),
            shared_arc_bool(false),
            shared_arc_bool(false),
        )
        .await
        .expect("extract succeeds");
        assert_eq!(entries, 2);
        assert_eq!(std::fs::read(dst.join("input/a.txt")).unwrap(), b"hello-a");
        assert_eq!(std::fs::read(dst.join("input/b.txt")).unwrap(), b"hello-b");
    }

    #[tokio::test]
    async fn extract_targz_to_local_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("input");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("a.txt"), b"hello-a").unwrap();
        std::fs::write(src_dir.join("b.txt"), b"hello-b").unwrap();

        let archive = tmp.path().join("out.tar.gz");
        let (tx, _rx) = mpsc::unbounded_channel();
        compress_pipeline(
            &ep(),
            vec![src_dir.clone()],
            &ep(),
            &archive,
            super::super::job::ArchiveFormat::TarGz,
            6,
            &tx,
            Uuid::new_v4(),
            shared_arc_bool(false),
            shared_arc_bool(false),
            Arc::new(AtomicU64::new(0)),
        )
        .await
        .expect("targz compress succeeds");

        let dst = tmp.path().join("out");
        std::fs::create_dir_all(&dst).unwrap();
        let entries = extract_pipeline(
            &ep(),
            &archive,
            &ep(),
            &dst,
            super::super::job::ArchiveFormat::TarGz,
            &tx,
            Uuid::new_v4(),
            shared_arc_bool(false),
            shared_arc_bool(false),
        )
        .await
        .expect("extract succeeds");
        assert_eq!(entries, 2);
        assert_eq!(std::fs::read(dst.join("input/a.txt")).unwrap(), b"hello-a");
    }

    #[tokio::test]
    async fn extract_7z_to_local_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("input");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("a.txt"), b"hello-a").unwrap();
        std::fs::write(src_dir.join("b.txt"), b"hello-b").unwrap();

        let archive = tmp.path().join("out.7z");
        let (tx, _rx) = mpsc::unbounded_channel();
        compress_pipeline(
            &ep(),
            vec![src_dir.clone()],
            &ep(),
            &archive,
            super::super::job::ArchiveFormat::SevenZ,
            5,
            &tx,
            Uuid::new_v4(),
            shared_arc_bool(false),
            shared_arc_bool(false),
            Arc::new(AtomicU64::new(0)),
        )
        .await
        .expect("sevenz compress succeeds");

        let dst = tmp.path().join("out");
        std::fs::create_dir_all(&dst).unwrap();
        let entries = extract_pipeline(
            &ep(),
            &archive,
            &ep(),
            &dst,
            super::super::job::ArchiveFormat::SevenZ,
            &tx,
            Uuid::new_v4(),
            shared_arc_bool(false),
            shared_arc_bool(false),
        )
        .await
        .expect("extract succeeds");
        assert_eq!(entries, 2);
        assert_eq!(std::fs::read(dst.join("input/a.txt")).unwrap(), b"hello-a");
    }

    #[test]
    fn sanitise_entry_name_rejects_unsafe_names() {
        // Safe names
        assert!(sanitise_entry_name("a/b/c.txt").is_ok());
        assert!(sanitise_entry_name("a\\b\\c.txt").is_ok());
        assert!(sanitise_entry_name("./relative/file").is_ok());
        // Path traversal variants
        assert!(sanitise_entry_name("../etc/passwd").is_err());
        assert!(sanitise_entry_name("/etc/passwd").is_err());
        assert!(sanitise_entry_name("a/../../b").is_err());
        assert!(sanitise_entry_name("a\0b").is_err());
        assert!(sanitise_entry_name("").is_err());
        // Drive-prefix variants (security review finding M3).
        // These pass the leading-/\\ check on Windows but
        // are absolute and would escape `dst_dir` after
        // `Path::join`.
        #[cfg(windows)]
        {
            assert!(sanitise_entry_name("C:/foo/bar.txt").is_err());
            assert!(sanitise_entry_name("C:\\foo\\bar.txt").is_err());
            assert!(sanitise_entry_name("C:foo").is_err());
            // UNC and device paths
            assert!(sanitise_entry_name("//server/share/file").is_err());
            assert!(sanitise_entry_name("\\\\?\\C:\\foo").is_err());
        }
    }
}
