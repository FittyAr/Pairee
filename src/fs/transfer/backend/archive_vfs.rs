//! Transfers that start or end inside an archive file (Strategy picked by
//! the engine from the job's paths): copying out extracts the selected
//! entries through the safe extractor, copying in and deleting rewrite the
//! archive in place (zip, tar and 7z). A move is the copy followed by the
//! removal of what was copied (see [`super::archive_move`]). Other
//! combinations are refused with a message.

use super::super::conflict_resolver::{ConflictAction, ConflictResolver};
use super::super::conflict_slot::ConflictSlot;
use super::super::job::{TransferJob, TransferOperation, TransferResults};
use super::super::options::TransferOptions;
use super::ops_jobs::archive::run_archive_blocking;
use crate::config::localization::t;
use crate::fs::archive::{
    ArchiveEdit, ArchiveVfs, EntrySource, extract_selected, split_archive_path,
};
use crate::fs::progress::ArchiveReport;
use crate::fs::transfer::control::JobControl;
use crate::fs::transfer::scan::guard::is_within;
use crate::fs::transfer::worker::is_destination_parent_dir;
use crate::fs::vfs::{LocalVfs, Vfs};
use crate::fs::walk::{WalkError, WalkOptions, Walker};
use anyhow::anyhow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// What an archive job does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchivePlan {
    /// Copy `selected` (paths inside `archive`) out to `dest`, relative to
    /// the archive folder `base`; a move then removes what was extracted.
    Extract {
        archive: PathBuf,
        base: PathBuf,
        selected: Vec<PathBuf>,
        dest: PathBuf,
        moving: bool,
    },
    /// Copy local `sources` into the archive folder `inner`; a move then
    /// deletes what was added.
    Import {
        archive: PathBuf,
        inner: PathBuf,
        sources: Vec<PathBuf>,
        moving: bool,
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

/// `inner` is (`itself`) or lies below an entry named like an archive: an
/// archive inside the archive, which jobs cannot reach.
fn in_nested(inner: &Path, itself: bool) -> bool {
    inner
        .ancestors()
        .skip(usize::from(!itself))
        .any(|a| !a.as_os_str().is_empty() && crate::fs::archive::is_browsable(a))
}

/// The archive plan for `job`, `None` when no path of it is inside an
/// archive (or it runs on a remote connection), or the reason it cannot run.
pub fn plan(job: &TransferJob) -> Option<Result<ArchivePlan, String>> {
    if job.remote.is_some() || !job.operation.uses_local_worker() {
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
    let nested_source = sources.iter().flatten().any(|(_, i)| in_nested(i, false));
    if nested_source
        || into_archive
            .as_ref()
            .is_some_and(|(_, i)| in_nested(i, true))
    {
        return refused();
    }
    let moving = job.operation == TransferOperation::Move;
    match (job.operation, from_archive, into_archive) {
        (TransferOperation::Delete, true, _) => {
            let Some((archive, inner)) = same_archive(&sources) else {
                return refused();
            };
            Some(Ok(ArchivePlan::Remove { archive, inner }))
        }
        (TransferOperation::Copy | TransferOperation::Move, true, None) => {
            let Some((archive, inner)) = same_archive(&sources) else {
                return refused();
            };
            if moving && !super::archive_move::editable(&archive) {
                return refused();
            }
            let base = inner[0].parent().unwrap_or(Path::new("")).to_path_buf();
            Some(Ok(ArchivePlan::Extract {
                archive,
                base,
                selected: inner,
                dest: destination,
                moving,
            }))
        }
        (TransferOperation::Copy | TransferOperation::Move, false, Some((archive, inner))) => {
            // A folder copied into an archive inside it would pack the
            // archive into itself, and a move would then delete it.
            if let Some(src) = job
                .sources
                .iter()
                .find(|src| is_within(&LocalVfs, &archive, src))
            {
                let reason = t("error_copy_into_itself").replacen("{}", &src.to_string_lossy(), 1);
                return Some(Err(reason));
            }
            Some(Ok(ArchivePlan::Import {
                archive,
                inner,
                sources: job.sources.clone(),
                moving,
            }))
        }
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
            moving,
        } => {
            let total = selected.len();
            run_archive_blocking(
                control,
                total,
                move |report| {
                    extract_selected(&archive, &base, &selected, &dest, report)?;
                    if !moving {
                        return Ok(());
                    }
                    super::archive_move::remove_extracted(&archive, &base, &selected, report)
                },
                "error_extraction_failed",
            )
            .await
        }
        ArchivePlan::Import {
            archive,
            inner,
            sources,
            moving,
        } => {
            let mut edit = import_edit(&inner, &sources).map_err(|e| anyhow!(e.to_string()))?;
            let mut skipped = TransferResults::default();
            resolve_conflicts(&archive, &mut edit, &conflicts, &control, &mut skipped).await?;
            let total = edit.add.len();
            let mut results = run_archive_blocking(
                control,
                total,
                move |report| {
                    apply_edit(&archive, &edit, report)?;
                    if moving {
                        super::archive_move::delete_imported(&edit, report);
                    }
                    Ok(())
                },
                "archive_update_failed",
            )
            .await?;
            results.skipped_files.extend(skipped.skipped_files);
            Ok(results)
        }
        ArchivePlan::Remove { archive, inner } => {
            let total = inner.len();
            let edit = ArchiveEdit {
                remove: inner,
                ..ArchiveEdit::default()
            };
            run_archive_blocking(
                control,
                total,
                move |report| apply_edit(&archive, &edit, report),
                "archive_update_failed",
            )
            .await
        }
    }
}

/// Every local file and folder of `sources`, placed below `inner`. An
/// unreadable folder fails the import instead of leaving files out.
fn import_edit(inner: &Path, sources: &[PathBuf]) -> Result<ArchiveEdit, WalkError> {
    let mut add = Vec::new();
    for src in sources {
        let top = inner.join(crate::fs::file_name_lossy(src));
        if !src.is_dir() {
            add.push((top, EntrySource::File(src.clone())));
            continue;
        }
        add.push((top.clone(), EntrySource::File(src.clone())));
        for item in Walker::new(&LocalVfs, src, WalkOptions::default()) {
            let entry = item?.entry;
            let Ok(rel) = entry.path.strip_prefix(src) else {
                continue;
            };
            add.push((top.join(rel), EntrySource::File(entry.path.clone())));
        }
    }
    Ok(ArchiveEdit {
        add,
        ..ArchiveEdit::default()
    })
}

/// Applies the job's conflict setting to the files of `edit` that already
/// exist in `archive`: skipped files are dropped, renamed ones get a free
/// name, overwritten ones replace the old entry when the archive is
/// rewritten.
async fn resolve_conflicts(
    archive: &Path,
    edit: &mut ArchiveEdit,
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
        src_fs: &LocalVfs,
        dst_fs: &vfs,
        auto: None,
    };
    let mut kept = Vec::with_capacity(edit.add.len());
    for (rel, source) in std::mem::take(&mut edit.add) {
        let mut dst = archive.join(&rel);
        let EntrySource::File(src) = &source else {
            kept.push((rel, source));
            continue;
        };
        if src.is_dir() || !vfs.exists(&dst) {
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
pub(super) fn apply_edit(
    archive: &Path,
    edit: &ArchiveEdit,
    report: &mut ArchiveReport,
) -> anyhow::Result<()> {
    let vfs = ArchiveVfs::open(archive.to_path_buf())
        .ok_or_else(|| anyhow!(t("vfs_action_unsupported")))?;
    report.ensure_running()?;
    let total_files = edit.add.len();
    let mut done = 0usize;
    vfs.edit(edit, &mut |rel| {
        report.ensure_running().map_err(std::io::Error::other)?;
        report.entry(&crate::fs::file_name_lossy(rel), done, total_files);
        done += 1;
        Ok(())
    })?;
    Ok(())
}
