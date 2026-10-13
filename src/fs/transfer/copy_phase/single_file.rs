//! One local file of a copy or move, written atomically: a link is
//! recreated as a link; anything else is cloned (copy-on-write) or streamed
//! into the destination's `.pairee-part` file — resuming what an earlier
//! attempt left there —, read back and verified when asked, and only then
//! renamed over the destination (the replaced file going to the trash when
//! the options say so).

use super::super::control::JobControl;
use super::super::link_copy::recreate_link;
use super::super::options::TransferOptions;
use super::super::part::{commit, discard, part_path};
use super::super::pipeline::copy_file_pipelined;
use super::super::verify::hash_file_on_disk;
use super::retry::{Committed, TransferOutcome, with_retries};
use super::verify_step;
use crate::config::localization::t;
use anyhow::anyhow;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Copies `src` to `dst`; `Err` only when the job is cancelled.
pub async fn transfer_one_file(
    src: &Path,
    dst: &Path,
    options: &TransferOptions,
    ctl: &JobControl,
    copied: &Arc<AtomicU64>,
) -> Result<TransferOutcome, anyhow::Error> {
    if src.is_symlink() && !options.follow_symlinks {
        let mut outcome = TransferOutcome::default();
        match recreate_link(src, dst, options.trash_replaced) {
            Ok(replaced) => {
                outcome.success = true;
                outcome.replaced = replaced;
            }
            Err(e) => {
                outcome.last_error = t("transfer_link_failed").replacen("{}", &e.to_string(), 1)
            }
        }
        return Ok(outcome);
    }
    let part = part_path(dst);
    with_retries(
        options,
        ctl,
        copied,
        || attempt(src, dst, &part, options, ctl, copied),
        || discard(&part),
    )
    .await
}

/// One attempt: fill `part`, verify it, commit it over `dst`.
async fn attempt(
    src: &Path,
    dst: &Path,
    part: &Path,
    options: &TransferOptions,
    ctl: &JobControl,
    copied: &Arc<AtomicU64>,
) -> anyhow::Result<Committed> {
    let src_hash = fill_part(src, part, options, ctl, copied).await?;
    let dst_hash = verify_part(src, part, src_hash.as_deref(), options, ctl).await?;
    let (part, dst, trash) = (
        part.to_path_buf(),
        dst.to_path_buf(),
        options.trash_replaced,
    );
    let replaced = blocking(move || commit(&part, &dst, trash)).await?;
    Ok(Committed {
        src_hash,
        dst_hash,
        replaced,
    })
}

/// Runs blocking filesystem work off the async threads.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> std::io::Result<T> + Send + 'static,
) -> anyhow::Result<T> {
    Ok(tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| anyhow!("copy task join error: {e}"))??)
}

/// Writes the whole of `src` into `part`; returns the source hash when
/// verifying.
async fn fill_part(
    src: &Path,
    part: &Path,
    options: &TransferOptions,
    ctl: &JobControl,
    copied: &Arc<AtomicU64>,
) -> anyhow::Result<Option<String>> {
    if options.use_copy_on_write {
        let (s, p) = (src.to_path_buf(), part.to_path_buf());
        let cloned = blocking(move || {
            discard(&p);
            crate::fs::transfer::reflink::clone_file(&s, &p)
        })
        .await;
        if cloned.is_ok() {
            copied.fetch_add(std::fs::metadata(src)?.len(), Ordering::SeqCst);
            if !options.verify_after_copy {
                return Ok(None);
            }
            return Ok(Some(hash_on_disk(src, options, ctl).await?));
        }
    }
    let (s, p, c) = (src.to_path_buf(), part.to_path_buf(), ctl.clone());
    let offset = blocking(move || crate::fs::transfer::resume::resume_offset(&s, &p, &c)).await?;
    copy_file_pipelined(src, part, offset, options, ctl, Arc::clone(copied)).await
}

async fn hash_on_disk(
    path: &Path,
    options: &TransferOptions,
    ctl: &JobControl,
) -> anyhow::Result<String> {
    let (path, algorithm, direct, ctl) = (
        path.to_path_buf(),
        options.hash_algorithm,
        options.direct_io,
        ctl.clone(),
    );
    blocking(move || hash_file_on_disk(&path, algorithm, direct, &|| ctl.is_cancelled())).await
}

/// Reads `part` back from the disk and compares its hash with the source's.
async fn verify_part(
    src: &Path,
    part: &Path,
    src_hash: Option<&str>,
    options: &TransferOptions,
    ctl: &JobControl,
) -> anyhow::Result<Option<String>> {
    if !options.verify_after_copy {
        return Ok(None);
    }
    verify_step::started(ctl, src, options);
    let dst_hash = hash_on_disk(part, options, ctl).await?;
    let size = std::fs::metadata(part).map_or(0, |m| m.len());
    verify_step::compare(ctl, size, src_hash, dst_hash)
}
