//! Single-file copy as a two-stage pipeline: a reader thread feeds blocks
//! (hashing the source) through a bounded channel to a writer thread
//! (hashing the destination and reporting progress). Both stages use Direct
//! I/O when requested, falling back to buffered I/O transparently.

use anyhow::anyhow;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use super::control::JobControl;
use super::direct_io::{AlignedBuffer, open_reader_direct, open_writer_direct, to_long_path};
use super::events::TransferEvent;
use super::hash::{HashStrategy, create_hasher};
use super::options::TransferOptions;

/// Direct I/O block alignment.
const ALIGN: usize = 4096;
/// Blocks in flight between reader and writer (backpressure).
const CHANNEL_BLOCKS: usize = 4;
/// Minimum interval between progress events of one file.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(150);

/// Copies `src` to `dst`. Returns the (source, destination) hashes when
/// verification is enabled.
pub async fn copy_file_pipelined(
    src: &Path,
    dst: &Path,
    options: &TransferOptions,
    ctl: &JobControl,
    bytes_transferred_acc: Arc<AtomicU64>,
) -> Result<(Option<String>, Option<String>), anyhow::Error> {
    let src = to_long_path(src);
    let dst = to_long_path(dst);
    let file_size = std::fs::metadata(&src)?.len();
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let (block_tx, block_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_BLOCKS);
    let reader = Reader {
        path: src,
        file_size,
        options: options.clone(),
        ctl: ctl.clone(),
    };
    let writer = Writer {
        path: dst,
        file_size,
        options: options.clone(),
        ctl: ctl.clone(),
        bytes_acc: bytes_transferred_acc,
    };
    let reader_handle = tokio::task::spawn_blocking(move || reader.run(block_tx));
    let writer_handle = tokio::task::spawn_blocking(move || writer.run(block_rx));

    let (reader_res, writer_res) = tokio::join!(reader_handle, writer_handle);
    let dst_hash = writer_res.map_err(|e| anyhow!("Writer task join error: {}", e))??;
    let src_hash = reader_res.map_err(|e| anyhow!("Reader task join error: {}", e))??;
    Ok((src_hash, dst_hash))
}

fn hasher_for(options: &TransferOptions) -> Option<Box<dyn HashStrategy>> {
    options
        .verify_after_copy
        .then(|| create_hasher(options.hash_algorithm))
}

/// Reader stage: reads blocks, hashes them and sends them to the writer.
struct Reader {
    path: PathBuf,
    file_size: u64,
    options: TransferOptions,
    ctl: JobControl,
}

impl Reader {
    fn run(self, block_tx: mpsc::Sender<Vec<u8>>) -> anyhow::Result<Option<String>> {
        let mut buf = AlignedBuffer::new(self.options.buffer_size.to_bytes(), ALIGN);
        let mut hasher = hasher_for(&self.options);
        let mut std_file = std::fs::File::open(&self.path)
            .map_err(|e| anyhow!("Error opening source file: {}", e))?;
        let mut direct = if self.options.direct_io {
            open_reader_direct(&self.path, true).ok()
        } else {
            None
        };

        let start = Instant::now();
        let mut offset = 0u64;
        loop {
            self.ctl.wait_if_paused_blocking()?;
            let remaining = self.file_size.saturating_sub(offset);
            // Direct I/O only for whole aligned blocks; anything else (or a
            // Direct I/O error) continues with buffered reads from `offset`.
            let direct_read = match direct.as_mut() {
                Some(f) if remaining >= ALIGN as u64 && remaining.is_multiple_of(ALIGN as u64) => {
                    f.read(buf.as_mut_slice()).ok()
                }
                _ => None,
            };
            let read = match direct_read {
                Some(n) => n,
                None => {
                    if direct.take().is_some() {
                        std_file.seek(SeekFrom::Start(offset))?;
                    }
                    std_file.read(buf.as_mut_slice())?
                }
            };
            if read == 0 {
                break;
            }
            let chunk = buf.as_slice()[..read].to_vec();
            if let Some(h) = hasher.as_mut() {
                h.update(&chunk);
            }
            if block_tx.blocking_send(chunk).is_err() {
                return Err(anyhow!("Writer thread disconnected"));
            }
            offset += read as u64;
            self.throttle(start, offset);
        }
        Ok(hasher.map(|h| h.finalize()))
    }

    /// Sleeps to keep the average rate under the bandwidth limit.
    fn throttle(&self, start: Instant, bytes: u64) {
        if let Some(rate) = self.options.limit_bandwidth_rate.filter(|&r| r > 0) {
            let expected = Duration::from_secs_f64(bytes as f64 / rate as f64);
            if let Some(ahead) = expected.checked_sub(start.elapsed()) {
                std::thread::sleep(ahead);
            }
        }
    }
}

/// Writer stage: writes received blocks, hashes them and reports progress.
struct Writer {
    path: PathBuf,
    file_size: u64,
    options: TransferOptions,
    ctl: JobControl,
    bytes_acc: Arc<AtomicU64>,
}

impl Writer {
    fn run(self, mut block_rx: mpsc::Receiver<Vec<u8>>) -> anyhow::Result<Option<String>> {
        let mut hasher = hasher_for(&self.options);
        let mut std_file = std::fs::File::create(&self.path)
            .map_err(|e| anyhow!("Error creating destination file: {}", e))?;
        let mut direct = if self.options.direct_io {
            open_writer_direct(&self.path, true).ok()
        } else {
            None
        };
        let mut buf = direct
            .is_some()
            .then(|| AlignedBuffer::new(self.options.buffer_size.to_bytes(), ALIGN));

        let mut written = 0u64;
        let mut last_progress = Instant::now();
        while let Some(chunk) = block_rx.blocking_recv() {
            self.ctl.wait_if_paused_blocking()?;
            let len = chunk.len();
            let wrote_direct = match (direct.as_mut(), buf.as_mut()) {
                (Some(f), Some(aligned)) if len >= ALIGN && len.is_multiple_of(ALIGN) => {
                    aligned.as_mut_slice()[..len].copy_from_slice(&chunk);
                    f.write_all(&aligned.as_slice()[..len]).is_ok()
                }
                _ => false,
            };
            if !wrote_direct {
                // Unaligned tail or Direct I/O error: continue buffered.
                if direct.take().is_some() {
                    std_file.seek(SeekFrom::Start(written))?;
                }
                std_file.write_all(&chunk)?;
            }
            written += len as u64;
            if let Some(h) = hasher.as_mut() {
                h.update(&chunk);
            }
            self.bytes_acc.fetch_add(len as u64, Ordering::SeqCst);
            if last_progress.elapsed() >= PROGRESS_INTERVAL {
                last_progress = Instant::now();
                self.ctl.emit(TransferEvent::FileProgress {
                    job_id: self.ctl.job_id,
                    bytes_copied: written,
                    bytes_total: self.file_size,
                });
            }
        }
        match direct {
            Some(f) => f.sync_all()?,
            None => std_file.sync_all()?,
        }
        Ok(hasher.map(|h| h.finalize()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::transfer::job::{TransferJob, TransferOperation};

    async fn copy_with(options: TransferOptions) {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let dst = dir.path().join("out").join("dst.bin");
        // Several aligned blocks plus an unaligned tail.
        let data: Vec<u8> = (0..(3 * ALIGN + 100)).map(|i| (i % 251) as u8).collect();
        std::fs::write(&src, &data).unwrap();
        let (tx, mut rx) = crate::fs::transfer::events::EventSender::channel();
        tokio::spawn(async move { while rx.recv().await.is_some() {} });
        let job = TransferJob::new(
            TransferOperation::Copy,
            vec![],
            dst.clone(),
            options.clone(),
        );
        let ctl = JobControl::for_job(&job, tx);
        let acc = Arc::new(AtomicU64::new(0));

        let (src_hash, dst_hash) =
            copy_file_pipelined(&src, &dst, &options, &ctl, Arc::clone(&acc))
                .await
                .unwrap();
        assert_eq!(std::fs::read(&dst).unwrap(), data);
        assert_eq!(acc.load(Ordering::SeqCst), data.len() as u64);
        assert_eq!(src_hash, dst_hash);
        assert_eq!(src_hash.is_some(), options.verify_after_copy);
    }

    #[tokio::test]
    async fn copies_buffered_and_direct_with_verification() {
        copy_with(TransferOptions::default()).await;
        copy_with(TransferOptions {
            verify_after_copy: true,
            direct_io: true,
            ..TransferOptions::default()
        })
        .await;
    }
}
