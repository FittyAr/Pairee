//! One file copied through the [`Vfs`] adapters (to or from an SFTP
//! server): streamed into `<name>.pairee-part` on the destination, flushed,
//! read back and verified when asked, then moved over the destination —
//! with the same retries, progress, bandwidth limit and cancellation as a
//! local copy. Times and permission bits are set with the adapter's
//! `setstat`.

use super::super::control::JobControl;
use super::super::events::TransferEvent;
use super::super::hash::{HashStrategy, create_hasher, hash_reader};
use super::super::options::TransferOptions;
use super::super::part::{Replaced, part_path};
use super::super::throttle::Throttle;
use super::Endpoints;
use super::retry::{Committed, TransferOutcome, with_retries};
use super::verify_step;
use anyhow::anyhow;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Copies `src` to `dst` between `fs.src` and `fs.dst`; `Err` only when the
/// job is cancelled.
pub async fn transfer_stream(
    fs: &Endpoints,
    src: &Path,
    dst: &Path,
    options: &TransferOptions,
    ctl: &JobControl,
    copied: &Arc<AtomicU64>,
) -> anyhow::Result<TransferOutcome> {
    let part = part_path(dst);
    let job = StreamJob {
        fs: fs.clone(),
        src: src.to_path_buf(),
        dst: dst.to_path_buf(),
        part: part.clone(),
        options: options.clone(),
        ctl: ctl.clone(),
        copied: Arc::clone(copied),
    };
    with_retries(
        options,
        ctl,
        copied,
        || {
            let job = job.clone();
            async move {
                tokio::task::spawn_blocking(move || job.attempt())
                    .await
                    .map_err(|e| anyhow!("copy task join error: {e}"))?
            }
        },
        || {
            let _ = fs.dst.remove_file(&part);
        },
    )
    .await
}

/// Copies the times and permission bits the options ask for from `src` to
/// `dst` (best effort).
pub fn copy_metadata(fs: &Endpoints, src: &Path, dst: &Path, options: &TransferOptions) {
    let modified = options
        .preserve_timestamps
        .then(|| fs.src.metadata(src).ok().and_then(|e| e.modified))
        .flatten();
    let mode = options
        .preserve_permissions
        .then(|| fs.src.attributes(src).ok().map(|a| a.mode))
        .flatten()
        .filter(|&m| m != 0);
    if modified.is_some() || mode.is_some() {
        let _ = fs.dst.set_metadata(dst, modified, mode);
    }
}

/// Everything one attempt needs, owned (it runs on a blocking thread).
#[derive(Clone)]
struct StreamJob {
    fs: Endpoints,
    src: PathBuf,
    dst: PathBuf,
    part: PathBuf,
    options: TransferOptions,
    ctl: JobControl,
    copied: Arc<AtomicU64>,
}

impl StreamJob {
    fn attempt(&self) -> anyhow::Result<Committed> {
        let result = self.write_part().and_then(|src_hash| self.commit(src_hash));
        if result.is_err() {
            let _ = self.fs.dst.remove_file(&self.part);
        }
        result
    }

    /// Streams the source into the temporary file; returns the source hash
    /// when verifying.
    fn write_part(&self) -> anyhow::Result<Option<String>> {
        let size = self.fs.src.metadata(&self.src).map_or(0, |e| e.size);
        let mut reader = ProgressRead {
            inner: self.fs.src.open_read(&self.src)?,
            ctl: &self.ctl,
            copied: &self.copied,
            hasher: self
                .options
                .verify_after_copy
                .then(|| create_hasher(self.options.hash_algorithm)),
            throttle: Throttle::new(self.options.limit_bandwidth_rate),
            done: 0,
            total: size,
        };
        let mut out = self.fs.dst.create_file(&self.part)?;
        io::copy(&mut reader, &mut out)?;
        out.flush()?;
        Ok(reader.hasher.map(|h| h.finalize()))
    }

    /// Verifies the temporary file (when asked) and moves it over the
    /// destination.
    fn commit(&self, src_hash: Option<String>) -> anyhow::Result<Committed> {
        let dst_hash = if self.options.verify_after_copy {
            verify_step::started(&self.ctl, &self.src, &self.options);
            let cancelled = || self.ctl.is_cancelled();
            let mut written = self.fs.dst.open_read(&self.part)?;
            let hash = hash_reader(&mut written, self.options.hash_algorithm, &cancelled)?;
            let size = self.fs.dst.stat(&self.part).map_or(0, |e| e.size);
            verify_step::compare(&self.ctl, size, src_hash.as_deref(), hash)?
        } else {
            None
        };
        let replaced = if self.fs.dst.stat(&self.dst).is_ok() {
            Replaced::Overwritten
        } else {
            Replaced::Nothing
        };
        self.fs.dst.replace(&self.part, &self.dst)?;
        Ok(Committed {
            src_hash,
            dst_hash,
            replaced,
        })
    }
}

/// The source stream of one attempt: honours pause and cancel, hashes,
/// paces to the bandwidth limit and reports progress.
struct ProgressRead<'a> {
    inner: Box<dyn Read + Send>,
    ctl: &'a JobControl,
    copied: &'a AtomicU64,
    hasher: Option<Box<dyn HashStrategy>>,
    throttle: Throttle,
    done: u64,
    total: u64,
}

impl Read for ProgressRead<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Not `Interrupted`: `io::copy` would retry that forever.
        self.ctl
            .wait_if_paused_blocking()
            .map_err(|e| io::Error::other(e.to_string()))?;
        let n = self.inner.read(buf)?;
        if let Some(h) = self.hasher.as_mut() {
            h.update(&buf[..n]);
        }
        self.done += n as u64;
        self.copied.fetch_add(n as u64, Ordering::SeqCst);
        self.throttle.pace(self.done);
        self.ctl.emit(TransferEvent::FileProgress {
            job_id: self.ctl.job_id,
            bytes_copied: self.done,
            bytes_total: self.total,
        });
        Ok(n)
    }
}
