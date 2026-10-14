//! The one byte-copy path of the transfer engine: a two-stage pipeline in
//! which a reader thread feeds blocks (hashing the source for verification
//! and pacing to the bandwidth limit) through a bounded channel to a writer
//! thread (reporting progress). Both stages honour pause and cancel.
//!
//! The ends are pluggable: a local copy reads with Direct I/O and writes
//! through [`local::LocalSink`] (Direct I/O, sparse files); a copy through
//! the [`crate::fs::vfs::Vfs`] adapters (SFTP, ...) reads the adapter's
//! stream and writes through [`StreamSink`]. Either way the writer fills
//! the temporary `.pairee-part` file of the destination (see
//! [`super::part`]); a copy resumed at `offset` keeps the bytes already
//! there and only streams the rest.

mod local;

pub use local::{LocalSink, local_source};

use anyhow::anyhow;
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use super::control::JobControl;
use super::events::TransferEvent;
use super::hash::{HashStrategy, create_hasher};
use super::options::TransferOptions;
use super::throttle::Throttle;

/// Blocks in flight between reader and writer (backpressure).
const CHANNEL_BLOCKS: usize = 4;
/// Minimum interval between progress events of one file.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(150);

/// Where the writer stage puts the blocks.
pub trait Sink: Send {
    fn write_block(&mut self, chunk: &[u8]) -> io::Result<()>;

    /// Makes everything written durable.
    fn finish(self: Box<Self>) -> io::Result<()>;
}

/// A writer of an adapter ([`crate::fs::vfs::Vfs::create_file`] or
/// [`crate::fs::vfs::Vfs::append_file`]); flushing makes it durable.
pub struct StreamSink(pub Box<dyn Write + Send>);

impl Sink for StreamSink {
    fn write_block(&mut self, chunk: &[u8]) -> io::Result<()> {
        self.0.write_all(chunk)
    }

    fn finish(mut self: Box<Self>) -> io::Result<()> {
        self.0.flush()
    }
}

/// Byte at which the source must be read for a copy resumed at `offset`:
/// the hash covers the whole file, so a verified copy re-reads the part it
/// does not send.
pub fn read_start(offset: u64, options: &TransferOptions) -> u64 {
    if options.verify_after_copy { 0 } else { offset }
}

/// One file to copy.
pub struct ByteCopy {
    /// The source, positioned at [`read_start`].
    pub source: Box<dyn Read + Send>,
    pub sink: Box<dyn Sink>,
    /// Bytes the sink already has (a resumed copy).
    pub offset: u64,
    /// Size of the source (progress).
    pub size: u64,
}

/// Copies `copy` with the job's buffer size, hash, bandwidth limit, pause
/// and progress. Returns the hash of the whole source when verification is
/// enabled.
pub async fn copy_pipelined(
    copy: ByteCopy,
    options: &TransferOptions,
    ctl: &JobControl,
    bytes_acc: Arc<AtomicU64>,
) -> anyhow::Result<Option<String>> {
    let (block_tx, block_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_BLOCKS);
    let size = copy.size;
    let reader = Reader {
        source: copy.source,
        start: read_start(copy.offset, options),
        offset: copy.offset,
        options: options.clone(),
        ctl: ctl.clone(),
    };
    let writer = Writer {
        sink: copy.sink,
        written: copy.offset,
        file_size: copy.size,
        ctl: ctl.clone(),
        bytes_acc,
    };
    let reader_handle = tokio::task::spawn_blocking(move || reader.run(block_tx));
    let writer_handle = tokio::task::spawn_blocking(move || writer.run(block_rx));
    let (reader_res, writer_res) = tokio::join!(reader_handle, writer_handle);
    let written = writer_res.map_err(|e| anyhow!("Writer task join error: {}", e))??;
    let hash = reader_res.map_err(|e| anyhow!("Reader task join error: {}", e))??;
    if written < size {
        // A connection that ended early (or a file that shrank): the copy
        // is incomplete and must never be committed (a move would then
        // delete the source).
        return Err(anyhow!(
            crate::config::localization::t("transfer_source_truncated")
                .replacen("{}", &written.to_string(), 1)
                .replacen("{}", &size.to_string(), 1)
        ));
    }
    Ok(hash)
}

/// Reader stage: reads blocks, hashes them and sends them to the writer.
struct Reader {
    source: Box<dyn Read + Send>,
    /// Byte the source is positioned at.
    start: u64,
    /// Bytes the writer already has.
    offset: u64,
    options: TransferOptions,
    ctl: JobControl,
}

impl Reader {
    fn run(mut self, block_tx: mpsc::Sender<Vec<u8>>) -> anyhow::Result<Option<String>> {
        let mut hasher: Option<Box<dyn HashStrategy>> = self
            .options
            .verify_after_copy
            .then(|| create_hasher(self.options.hash_algorithm));
        let mut buf = vec![0u8; self.options.buffer_size.to_bytes()];
        let mut throttle = Throttle::new(self.options.limit_bandwidth_rate);
        let mut pos = self.start;
        loop {
            self.ctl.wait_if_paused_blocking()?;
            let read = self.source.read(&mut buf)?;
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
            let paced = pos.saturating_sub(self.offset.max(self.start));
            if let Some(rate) = self.ctl.live.bandwidth_rate() {
                throttle.retarget(rate, paced);
            }
            throttle.pace(paced, &|| self.ctl.is_cancelled());
        }
        Ok(hasher.map(|h| h.finalize()))
    }
}

/// Writer stage: writes received blocks and reports progress.
struct Writer {
    sink: Box<dyn Sink>,
    /// Bytes in the destination so far.
    written: u64,
    file_size: u64,
    ctl: JobControl,
    bytes_acc: Arc<AtomicU64>,
}

impl Writer {
    /// Returns the bytes the destination holds.
    fn run(mut self, mut block_rx: mpsc::Receiver<Vec<u8>>) -> anyhow::Result<u64> {
        // Resumed bytes count as copied.
        self.bytes_acc.fetch_add(self.written, Ordering::SeqCst);
        let mut last_progress = Instant::now();
        while let Some(chunk) = block_rx.blocking_recv() {
            self.ctl.wait_if_paused_blocking()?;
            self.sink.write_block(&chunk)?;
            self.written += chunk.len() as u64;
            self.bytes_acc
                .fetch_add(chunk.len() as u64, Ordering::SeqCst);
            if last_progress.elapsed() >= PROGRESS_INTERVAL {
                last_progress = Instant::now();
                self.emit_progress();
            }
        }
        self.emit_progress();
        self.sink.finish()?;
        Ok(self.written)
    }

    fn emit_progress(&self) {
        self.ctl.emit(TransferEvent::FileProgress {
            job_id: self.ctl.job_id,
            bytes_copied: self.written,
            bytes_total: self.file_size,
        });
    }
}

#[cfg(test)]
mod tests;
