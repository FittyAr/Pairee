//! Transfers that start or end inside an archive file (Strategy picked by
//! the engine from the job's paths): copying out extracts the selected
//! entries through the safe extractor, copying in and deleting rewrite a
//! zip archive in place. Other combinations are refused with a message.

use super::super::conflict_resolver::{ConflictAction, ConflictResolver};
use super::super::conflict_slot::ConflictSlot;
use super::super::job::{TransferJob, TransferOperation, TransferResults};
use super::super::options::TransferOptions;
use super::ops_jobs::archive::run_archive_blocking;
use crate::config::localization::t;
use crate::fs::archive::{ArchiveVfs, ZipEdit, ZipSource, extract_selected, split_archive_path};
use crate::fs::progress::{ProgressUpdate, ensure_not_cancelled};
use crate::fs::transfer::control::JobControl;
use crate::fs::vfs::{LocalVfs, Vfs};
use anyhow::anyhow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// What an archive job does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchivePlan {
    /// Copy `selected` (paths inside `archive`) out to `dest`, relative to
    /// the archive folder `base`.
    Extract {
        archive: PathBuf,
        base: PathBuf,
        selected: Vec<PathBuf>,
        dest: PathBuf,
    },
    /// Copy local `sources` into the archive folder `inner`.
    Import {
        archive: PathBuf,
        inner: PathBuf,
        sources: Vec<PathBuf>,
    },
    /// Delete `inner` paths (with everything below them) from `archive`.
    Remove {
        archive: PathBuf,
        inner: Vec<PathBuf>,
    },
}

/// The archive and inner path of `path` when it lies strictly inside one.
fn inside(path: &Path) -> Option<(PathBuf, PathBuf)> {
    split_archive_path(path).filter(|(_, inner)| !inner.as_os_str().is_empty())
}

/// The archive plan for `job`, `None` when no path of it is inside an
/// archive (or it runs over SSH), or the reason it cannot run.
pub fn plan(job: &TransferJob) -> Option<Result<ArchivePlan, String>> {
    if job.ssh.is_some() || !job.operation.uses_local_worker() {
        return None;
    }
    let sources: Vec<_> = job.sources.iter().map(|s| inside(s)).collect();
    let from_archive = sources.iter().any(Option::is_some);
    let into_archive = (job.operation != TransferOperation::Delete)
        .then(|| split_archive_path(&job.destination))
        .flatten();
    if !from_archive && into_archive.is_none() {
        return None;
    }
    let refused = || Some(Err(t("vfs_action_unsupported")));
    match (job.operation, from_archive, into_archive) {
        (TransferOperation::Copy, true, None) | (TransferOperation::Delete, true, _) => {
            let Some((archive, inner)) = same_archive(&sources) else {
                return refused();
            };
            Some(Ok(if job.operation == TransferOperation::Delete {
                ArchivePlan::Remove { archive, inner }
            } else {
                let base = inner[0].parent().unwrap_or(Path::new("")).to_path_buf();
                ArchivePlan::Extract {
                    archive,
                    base,
                    selected: inner,
                    dest: job.destination.clone(),
                }
            }))
        }
        (TransferOperation::Copy, false, Some((archive, inner))) => Some(Ok(ArchivePlan::Import {
            archive,
            inner,
            sources: job.sources.clone(),
        })),
        _ => refused(),
    }
}

/// The archive shared by every source and their inner paths.
fn same_archive(sources: &[Option<(PathBuf, PathBuf)>]) -> Option<(PathBuf, Vec<PathBuf>)> {
    let archive = sources.first()?.as_ref()?.0.clone();
    let inner = sources
        .iter()
        .map(|s| {
            s.as_ref()
                .filter(|(a, _)| *a == archive)
                .map(|(_, i)| i.clone())
        })
        .collect::<Option<Vec<_>>>()?;
    Some((archive, inner))
}

/// How a job resolves existing destinations (its options and the slot the
/// conflict dialog answers through).
pub struct ConflictSetup {
    pub options: TransferOptions,
    pub slot: Arc<ConflictSlot>,
}

impl ConflictSetup {
    pub fn of(job: &TransferJob) -> Self {
        Self {
            options: job.options.clone(),
            slot: Arc::clone(&job.active_conflict),
        }
    }
}

/// Runs `plan` on the blocking pool with Transfer Engine progress.
pub async fn run(
    plan: ArchivePlan,
    control: JobControl,
    conflicts: ConflictSetup,
) -> anyhow::Result<TransferResults> {
    match plan {
        ArchivePlan::Extract {
            archive,
            base,
            selected,
            dest,
        } => {
            let total = selected.len();
            run_archive_blocking(
                control,
                total,
                move |tx, cancel| extract_selected(&archive, &base, &selected, &dest, tx, cancel),
                "error_extraction_failed",
            )
            .await
        }
        ArchivePlan::Import {
            archive,
            inner,
            sources,
        } => {
            let mut edit = import_edit(&inner, &sources);
            let mut skipped = TransferResults::default();
            resolve_conflicts(&archive, &mut edit, &conflicts, &control, &mut skipped).await?;
            let total = edit.add.len();
            let mut results = run_archive_blocking(
                control,
                total,
                move |tx, cancel| apply_edit(&archive, &edit, tx, cancel),
                "archive_update_failed",
            )
            .await?;
            results.skipped_files.extend(skipped.skipped_files);
            Ok(results)
        }
        ArchivePlan::Remove { archive, inner } => {
            let total = inner.len();
            let edit = ZipEdit {
                remove: inner,
                ..ZipEdit::default()
            };
            run_archive_blocking(
                control,
                total,
                move |tx, cancel| apply_edit(&archive, &edit, tx, cancel),
                "archive_update_failed",
            )
            .await
        }
    }
}

/// Every local file and folder of `sources`, placed below `inner`.
fn import_edit(inner: &Path, sources: &[PathBuf]) -> ZipEdit {
    let mut add = Vec::new();
    for src in sources {
        let top = inner.join(crate::fs::file_name_lossy(src));
        if !src.is_dir() {
            add.push((top, ZipSource::File(src.clone())));
            continue;
        }
        add.push((top.clone(), ZipSource::Dir));
        for entry in LocalVfs.walk(src) {
            let Ok(rel) = entry.path.strip_prefix(src) else {
                continue;
            };
            let source = if entry.is_dir {
                ZipSource::Dir
            } else {
                ZipSource::File(entry.path.clone())
            };
            add.push((top.join(rel), source));
        }
    }
    ZipEdit {
        add,
        ..ZipEdit::default()
    }
}

/// Applies the job's conflict setting to the files of `edit` that already
/// exist in `archive`: skipped files are dropped, renamed ones get a free
/// name, overwritten ones replace the old entry when the archive is
/// rewritten.
async fn resolve_conflicts(
    archive: &Path,
    edit: &mut ZipEdit,
    setup: &ConflictSetup,
    control: &JobControl,
    results: &mut TransferResults,
) -> anyhow::Result<()> {
    let vfs = ArchiveVfs::open(archive.to_path_buf())
        .ok_or_else(|| anyhow!(t("vfs_action_unsupported")))?;
    let mut resolver = ConflictResolver {
        options: &setup.options,
        ctl: control,
        slot: &setup.slot,
        dst_fs: &vfs,
        auto: None,
    };
    let mut kept = Vec::with_capacity(edit.add.len());
    for (rel, source) in std::mem::take(&mut edit.add) {
        let mut dst = archive.join(&rel);
        let ZipSource::File(src) = &source else {
            kept.push((rel, source));
            continue;
        };
        if !vfs.exists(&dst) {
            kept.push((rel, source));
            continue;
        }
        if let ConflictAction::Proceed = resolver.resolve(src, &mut dst, results).await? {
            let rel = dst.strip_prefix(archive).map(Path::to_path_buf)?;
            kept.push((rel, source));
        }
    }
    edit.add = kept;
    Ok(())
}

/// Rewrites `archive` with `edit`, reporting each added entry.
fn apply_edit(
    archive: &Path,
    edit: &ZipEdit,
    tx: &tokio::sync::mpsc::Sender<ProgressUpdate>,
    cancel: &std::sync::atomic::AtomicBool,
) -> anyhow::Result<()> {
    let vfs = ArchiveVfs::open(archive.to_path_buf())
        .ok_or_else(|| anyhow!(t("vfs_action_unsupported")))?;
    ensure_not_cancelled(cancel)?;
    let total_files = edit.add.len();
    let mut done = 0usize;
    vfs.edit(edit, &mut |rel| {
        ensure_not_cancelled(cancel).map_err(std::io::Error::other)?;
        let _ = tx.blocking_send(ProgressUpdate {
            skipped: false,
            current_file: crate::fs::file_name_lossy(rel),
            files_copied: done,
            total_files,
            bytes_copied: 0,
            total_bytes: 0,
            error: None,
        });
        done += 1;
        Ok(())
    })?;
    Ok(())
}
