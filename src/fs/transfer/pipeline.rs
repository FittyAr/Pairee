//! Single-file copy as a two-stage pipeline: a reader thread feeds blocks
//! (hashing the source for verification) through a bounded channel to a
//! writer thread (reporting progress). Both stages use Direct I/O for whole
//! aligned blocks when requested, and buffered I/O for the rest.
//!
//! The writer fills the temporary `.pairee-part` file of the destination
//! (see [`super::part`]); a copy resumed at `offset` keeps the bytes already
//! there and only streams the rest.

use anyhow::anyhow;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use super::control::JobControl;
use super::direct_io::{ALIGN, AlignedBuffer, DiskReader, open_writer_direct, to_long_path};
use super::events::TransferEvent;
use super::hash::{HashStrategy, create_hasher};
use super::options::TransferOptions;
use super::sparse;
use super::throttle::Throttle;

/// Blocks in flight between reader and writer (backpressure).
const CHANNEL_BLOCKS: usize = 4;
/// Minimum interval between progress events of one file.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(150);

/// Copies `src` into `part` from byte `offset` (0: from scratch). Returns
/// the hash of the whole source when verification is enabled.
pub async fn copy_file_pipelined(
    src: &Path,
    part: &Path,
    offset: u64,
    options: &TransferOptions,
    ctl: &JobControl,
    bytes_transferred_acc: Arc<AtomicU64>,
) -> Result<Option<String>, anyhow::Error> {
    let src = to_long_path(src);
    let part = to_long_path(part);
    let file_size = std::fs::metadata(&src)?.len();
    if let Some(parent) = part.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let (block_tx, block_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_BLOCKS);
    let reader = Reader {
        path: src,
        offset,
        options: options.clone(),
        ctl: ctl.clone(),
    };
    let writer = Writer {
        path: part,
        offset,
        file_size,
        options: options.clone(),
        ctl: ctl.clone(),
        bytes_acc: bytes_transferred_acc,
    };
    let reader_handle = tokio::task::spawn_blocking(move || reader.run(block_tx));
    let writer_handle = tokio::task::spawn_blocking(move || writer.run(block_rx));

    let (reader_res, writer_res) = tokio::join!(reader_handle, writer_handle);
    writer_res.map_err(|e| anyhow!("Writer task join error: {}", e))??;
    reader_res.map_err(|e| anyhow!("Reader task join error: {}", e))?
}

fn hasher_for(options: &TransferOptions) -> Option<Box<dyn HashStrategy>> {
    options
        .verify_after_copy
        .then(|| create_hasher(options.hash_algorithm))
}

/// Reader stage: reads blocks, hashes them and sends them to the writer.
struct Reader {
    path: PathBuf,
    /// Bytes the writer already has.
    offset: u64,
    options: TransferOptions,
    ctl: JobControl,
}

impl Reader {
    fn run(self, block_tx: mpsc::Sender<Vec<u8>>) -> anyhow::Result<Option<String>> {
        let mut hasher = hasher_for(&self.options);
        // The hash covers the whole file, so a resumed copy re-reads the
        // part it does not send.
        let start = if hasher.is_some() { 0 } else { self.offset };
        let block = self.options.buffer_size.to_bytes();
        let mut file = DiskReader::open(&self.path, self.options.direct_io, start, block)
            .map_err(|e| anyhow!("Error opening source file: {}", e))?;
        let mut buf = vec![0u8; block];
        let throttle = Throttle::new(self.options.limit_bandwidth_rate);
        let mut pos = start;
        loop {
            self.ctl.wait_if_paused_blocking()?;
            let read = file.read(&mut buf)?;
            if read == 0 {
                break;
            }
            let chunk = &buf[..read];
            if let Some(h) = hasher.as_mut() {
                h.update(chunk);
            }
            let skip = self.offset.saturating_sub(pos).min(read as u64) as usize;
            if skip < read && block_tx.blocking_send(chunk[skip..].to_vec()).is_err() {
                return Err(anyhow!("Writer thread disconnected"));
            }
            pos += read as u64;
            throttle.pace(pos.saturating_sub(self.offset.max(start)));
        }
        Ok(hasher.map(|h| h.finalize()))
    }
}

/// Writer stage: writes received blocks and reports progress.
struct Writer {
    path: PathBuf,
    /// Bytes already in the file (a resumed copy).
    offset: u64,
    file_size: u64,
    options: TransferOptions,
    ctl: JobControl,
    bytes_acc: Arc<AtomicU64>,
}

/// Where the writer puts its blocks.
struct Output {
    file: File,
    /// Direct I/O handle, used while blocks stay whole and aligned.
    direct: Option<File>,
    aligned: Option<AlignedBuffer>,
    sparse: bool,
    written: u64,
}

impl Output {
    fn write(&mut self, chunk: &[u8]) -> std::io::Result<()> {
        let len = chunk.len();
        if let (Some(f), Some(buf)) = (self.direct.as_mut(), self.aligned.as_mut())
            && len >= ALIGN
            && len.is_multiple_of(ALIGN)
        {
            buf.as_mut_slice()[..len].copy_from_slice(chunk);
            if f.write_all(&buf.as_slice()[..len]).is_ok() {
                self.written += len as u64;
                return Ok(());
            }
        }
        if self.direct.take().is_some() {
            // Unaligned tail or Direct I/O error: continue buffered.
            self.file.seek(SeekFrom::Start(self.written))?;
        }
        if self.sparse {
            sparse::write_sparse(&mut self.file, chunk)?;
        } else {
            self.file.write_all(chunk)?;
        }
        self.written += len as u64;
        Ok(())
    }

    fn finish(self) -> std::io::Result<()> {
        if self.sparse {
            // A trailing hole is only a seek: give the file its length.
            self.file.set_len(self.written)?;
        }
        match self.direct {
            Some(f) => f.sync_all(),
            None => self.file.sync_all(),
        }
    }
}

impl Writer {
    fn open(&self) -> anyhow::Result<Output> {
        let mut file = if self.offset > 0 {
            OpenOptions::new().write(true).open(&self.path)
        } else {
            File::create(&self.path)
        }
        .map_err(|e| anyhow!("Error creating destination file: {}", e))?;
        if self.offset > 0 {
            file.set_len(self.offset)?;
            file.seek(SeekFrom::Start(self.offset))?;
        }
        // Holes need the sparse flag (Windows); without it zeros are written.
        let sparse = self.options.produce_sparse_files && sparse::mark_sparse(&file).is_ok();
        // Direct I/O opens (and truncates) the file again: only for a fresh,
        // non-sparse copy.
        let direct = (self.options.direct_io && self.offset == 0 && !sparse)
            .then(|| open_writer_direct(&self.path, true).ok())
            .flatten();
        let aligned = direct
            .is_some()
            .then(|| AlignedBuffer::new(self.options.buffer_size.to_bytes(), ALIGN));
        Ok(Output {
            file,
            direct,
            aligned,
            sparse,
            written: self.offset,
        })
    }

    fn run(self, mut block_rx: mpsc::Receiver<Vec<u8>>) -> anyhow::Result<()> {
        let mut out = self.open()?;
        // Resumed bytes count as copied.
        self.bytes_acc.fetch_add(self.offset, Ordering::SeqCst);
        let mut last_progress = Instant::now();
        while let Some(chunk) = block_rx.blocking_recv() {
            self.ctl.wait_if_paused_blocking()?;
            out.write(&chunk)?;
            self.bytes_acc
                .fetch_add(chunk.len() as u64, Ordering::SeqCst);
            if last_progress.elapsed() >= PROGRESS_INTERVAL {
                last_progress = Instant::now();
                self.ctl.emit(TransferEvent::FileProgress {
                    job_id: self.ctl.job_id,
                    bytes_copied: out.written,
                    bytes_total: self.file_size,
                });
            }
        }
        out.finish()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
