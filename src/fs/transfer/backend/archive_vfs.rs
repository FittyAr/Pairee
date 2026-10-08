//! Transfers that start or end inside an archive file (Strategy picked by
//! the engine from the job's paths): copying out extracts the selected
//! entries through the safe extractor, copying in and deleting rewrite a
//! zip archive in place. Other combinations are refused with a message.

use super::super::job::{TransferJob, TransferOperation, TransferResults};
use super::ops_jobs::archive::run_archive_blocking;
use crate::config::localization::t;
use crate::fs::archive::{ArchiveVfs, ZipEdit, ZipSource, extract_selected, split_archive_path};
use crate::fs::progress::{ProgressUpdate, ensure_not_cancelled};
use crate::fs::transfer::control::JobControl;
use crate::fs::transfer::worker::is_destination_parent_dir;
use crate::fs::vfs::{LocalVfs, Vfs};
use anyhow::anyhow;
use std::path::{Path, PathBuf};

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
    let destination = target_folder(job);
    let sources: Vec<_> = job.sources.iter().map(|s| inside(s)).collect();
    let from_archive = sources.iter().any(Option::is_some);
    let into_archive = (job.operation != TransferOperation::Delete)
        .then(|| split_archive_path(&destination))
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
                    dest: destination,
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

/// The folder the job's items go into. For a single item the transfer
/// dialog pre-fills `<folder>/<name>`, the target path itself (as local
/// copies read it); archive plans place items into a folder, so that
/// target stands for its parent.
fn target_folder(job: &TransferJob) -> PathBuf {
    let dest = &job.destination;
    let same_name = matches!(job.sources.as_slice(), [single]
        if dest.file_name().is_some() && dest.file_name() == single.file_name());
    match dest.parent() {
        Some(parent)
            if same_name && !is_destination_parent_dir(&job.sources, dest, |p| p.is_dir()) =>
        {
            parent.to_path_buf()
        }
        _ => dest.clone(),
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

/// Runs `plan` on the blocking pool with Transfer Engine progress.
pub async fn run(plan: ArchivePlan, control: JobControl) -> anyhow::Result<TransferResults> {
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
            let edit = import_edit(&inner, &sources);
            let total = edit.add.len();
            run_archive_blocking(
                control,
                total,
                move |tx, cancel| apply_edit(&archive, &edit, tx, cancel),
                "archive_update_failed",
            )
            .await
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
