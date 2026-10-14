//! One file of a copy or move, written atomically through the one byte
//! pipeline ([`crate::fs::transfer::pipeline`]) whatever the endpoints:
//!
//! 1. fill the destination's `.pairee-part` file — resuming what an earlier
//!    attempt left there (local or remote alike) —, or clone it
//!    (copy-on-write, local only);
//! 2. read it back and verify it when asked;
//! 3. move it over the destination (locally the replaced file can go to
//!    the trash; adapters use their atomic `replace`).
//!
//! Retries, progress, bandwidth limit, pause and cancel are the same for
//! every endpoint. What stays local-only is what only a local disk has:
//! recreating links, Direct I/O, sparse output, reflinks and the trash.

use super::super::control::JobControl;
use super::super::hash::hash_reader;
use super::super::link_copy::recreate_link;
use super::super::options::TransferOptions;
use super::super::part::{Replaced, commit, folder_in_the_way, part_path};
use super::super::pipeline::{
    ByteCopy, LocalSink, StreamSink, copy_pipelined, local_source, read_start,
};
use super::super::resume::{Side, resume_offset};
use super::super::verify::hash_file_on_disk;
use super::Endpoints;
use super::retry::{Committed, TransferOutcome, with_retries};
use super::verify_step;
use crate::config::localization::t;
use anyhow::anyhow;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Copies `src` to `dst` between `fs.src` and `fs.dst`; `Err` only when the
/// job is cancelled.
pub async fn transfer_file(
    fs: &Endpoints,
    src: &Path,
    dst: &Path,
    options: &TransferOptions,
    ctl: &JobControl,
    copied: &Arc<AtomicU64>,
) -> anyhow::Result<TransferOutcome> {
    if fs.local && src.is_symlink() && !options.follow_symlinks {
        return Ok(copy_link(src, dst, options));
    }
    let file = FileCopy {
        fs: fs.clone(),
        src: src.to_path_buf(),
        dst: dst.to_path_buf(),
        part: part_path(dst),
        options: options.clone(),
        ctl: ctl.clone(),
        copied: Arc::clone(copied),
    };
    with_retries(
        options,
        ctl,
        copied,
        || file.clone().attempt(),
        || {
            let _ = file.fs.dst.remove_file(&file.part);
        },
    )
    .await
}

/// A local link recreated as a link.
fn copy_link(src: &Path, dst: &Path, options: &TransferOptions) -> TransferOutcome {
    let mut outcome = TransferOutcome::default();
    match recreate_link(src, dst, options.trash_replaced) {
        Ok(replaced) => {
            outcome.success = true;
            outcome.replaced = replaced;
        }
        Err(e) => outcome.last_error = t("transfer_link_failed").replacen("{}", &e.to_string(), 1),
    }
    outcome
}

/// Runs blocking filesystem work off the async threads.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> anyhow::Result<T> {
    Ok(tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| anyhow!("copy task join error: {e}"))??)
}

/// Everything one attempt needs, owned (its steps run on blocking threads).
#[derive(Clone)]
struct FileCopy {
    fs: Endpoints,
    src: PathBuf,
    dst: PathBuf,
    part: PathBuf,
    options: TransferOptions,
    ctl: JobControl,
    copied: Arc<AtomicU64>,
}

impl FileCopy {
    /// One attempt: fill the part file, verify it, commit it over `dst`.
    async fn attempt(self) -> anyhow::Result<Committed> {
        let src_hash = self.fill_part().await?;
        let dst_hash = self.verify_part(src_hash.as_deref()).await?;
        let this = self.clone();
        let replaced = blocking(move || this.commit()).await?;
        Ok(Committed {
            src_hash,
            dst_hash,
            replaced,
        })
    }

    /// Writes the whole source into the part file; returns the source hash
    /// when verifying.
    async fn fill_part(&self) -> anyhow::Result<Option<String>> {
        if self.fs.local && self.options.use_copy_on_write {
            let this = self.clone();
            if blocking(move || this.clone_part()).await.is_ok() {
                return self.cloned_hash().await;
            }
        }
        let this = self.clone();
        let copy = blocking(move || this.open_ends()).await?;
        copy_pipelined(copy, &self.options, &self.ctl, Arc::clone(&self.copied)).await
    }

    /// The copy-on-write clone of a local file.
    fn clone_part(&self) -> io::Result<()> {
        let _ = crate::fs::remove::remove_entry(&self.part);
        crate::fs::transfer::reflink::clone_file(&self.src, &self.part)?;
        let size = std::fs::metadata(&self.src)?.len();
        self.copied.fetch_add(size, Ordering::SeqCst);
        Ok(())
    }

    /// The source hash after a clone (nothing was streamed to hash).
    async fn cloned_hash(&self) -> anyhow::Result<Option<String>> {
        if !self.options.verify_after_copy {
            return Ok(None);
        }
        let (src, ctl, options) = (self.src.clone(), self.ctl.clone(), self.options.clone());
        let hash = blocking(move || {
            hash_file_on_disk(&src, options.hash_algorithm, options.direct_io, &|| {
                ctl.is_cancelled()
            })
        })
        .await?;
        Ok(Some(hash))
    }

    /// The source and the part file, resumed where an earlier attempt left
    /// it when its bytes still match the source.
    fn open_ends(&self) -> io::Result<ByteCopy> {
        let mut offset = resume_offset(
            Side {
                vfs: self.fs.src.as_ref(),
                path: &self.src,
            },
            Side {
                vfs: self.fs.dst.as_ref(),
                path: &self.part,
            },
            &self.ctl,
        )?;
        let sink: Box<dyn super::super::pipeline::Sink> = if self.fs.local {
            Box::new(LocalSink::open(&self.part, offset, &self.options)?)
        } else {
            let (writer, kept) = self.remote_part(offset)?;
            offset = kept;
            Box::new(StreamSink(writer))
        };
        let start = read_start(offset, &self.options);
        let (source, size) = if self.fs.local {
            local_source(&self.src, start, &self.options)?
        } else {
            let size = self.fs.src.metadata(&self.src)?.size;
            (self.fs.src.open_read_at(&self.src, start)?, size)
        };
        Ok(ByteCopy {
            source,
            sink,
            offset,
            size,
        })
    }

    /// The part file on an adapter: appended to when `offset` bytes are
    /// kept and the adapter can append, created afresh otherwise. Returns
    /// the writer and the bytes it keeps.
    fn remote_part(&self, offset: u64) -> io::Result<(Box<dyn io::Write + Send>, u64)> {
        let dst = self.fs.dst.as_ref();
        if offset > 0 {
            if let Ok(writer) = dst.append_file(&self.part) {
                return Ok((writer, offset));
            }
            let _ = dst.remove_file(&self.part);
        }
        Ok((dst.create_file(&self.part)?, 0))
    }

    /// Reads the part file back and compares its hash with the source's.
    async fn verify_part(&self, src_hash: Option<&str>) -> anyhow::Result<Option<String>> {
        if !self.options.verify_after_copy {
            return Ok(None);
        }
        verify_step::started(&self.ctl, &self.src, &self.options);
        let this = self.clone();
        let (dst_hash, size) = blocking(move || this.hash_part()).await?;
        verify_step::compare(&self.ctl, size, src_hash, dst_hash)
    }

    /// Hash and size of the part file, read from its storage.
    fn hash_part(&self) -> io::Result<(String, u64)> {
        let algorithm = self.options.hash_algorithm;
        let cancelled = || self.ctl.is_cancelled();
        let hash = if self.fs.local {
            hash_file_on_disk(&self.part, algorithm, self.options.direct_io, &cancelled)?
        } else {
            hash_reader(
                &mut self.fs.dst.open_read(&self.part)?,
                algorithm,
                &cancelled,
            )?
        };
        let size = self.fs.dst.stat(&self.part).map_or(0, |e| e.size);
        Ok((hash, size))
    }

    /// The part file on an adapter holds as many bytes as the source: what
    /// the server reports after the upload closed, not what was sent (a
    /// server that dropped data on close must never be committed over the
    /// destination, nor lead a move to delete the source).
    fn ensure_part_complete(&self) -> io::Result<()> {
        let expected = self.fs.src.metadata(&self.src)?.size;
        let stored = self.fs.dst.stat(&self.part)?.size;
        if stored == expected {
            return Ok(());
        }
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            crate::config::localization::t_fill(
                "transfer_part_size_mismatch",
                &[&stored.to_string(), &expected.to_string()],
            ),
        ))
    }

    /// Moves the part file over the destination.
    fn commit(&self) -> io::Result<Replaced> {
        if self.fs.local {
            return commit(&self.part, &self.dst, self.options.trash_replaced);
        }
        self.ensure_part_complete()?;
        // WebDAV's MOVE with overwrite deletes a folder at the target with
        // everything in it: a folder is never replaced, as locally.
        let replaced = match self.fs.dst.stat(&self.dst) {
            Ok(entry) if entry.is_dir => return Err(folder_in_the_way(&self.dst)),
            Ok(_) => Replaced::Overwritten,
            Err(_) => Replaced::Nothing,
        };
        self.fs.dst.replace(&self.part, &self.dst)?;
        Ok(replaced)
    }
}

/// Copies the metadata the options ask for from `src` to `dst` (best
/// effort): everything the platform has between local files, the times and
/// permission bits through the adapters otherwise.
pub fn copy_metadata(fs: &Endpoints, src: &Path, dst: &Path, options: &TransferOptions) {
    if fs.local {
        let _ = super::super::metadata::preserve_metadata(src, dst, options);
        return;
    }
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
