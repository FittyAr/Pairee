//! Transfers that start or end inside an archive file (Strategy picked by
//! the engine from the job's paths): copying out — and extracting a whole
//! archive — extracts the selected entries through the safe extractor
//! (existing files are resolved by the job's conflict setting first), copying in and deleting rewrite the
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
    ArchiveEdit, ArchiveVfs, EntrySource, Existing, Selection, extract_selected, split_archive_path,
};
use crate::fs::progress::ArchiveReport;
use crate::fs::transfer::control::JobControl;
use crate::fs::transfer::scan::guard::is_within;
use crate::fs::transfer::worker::is_destination_parent_dir;
use crate::fs::vfs::{LocalVfs, Vfs};
use crate::fs::walk::{WalkError, WalkOptions, Walker};
use anyhow::anyhow;
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// What an archive job does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchivePlan {
    /// Copy the entries `selection` takes out to `dest`; a move then
    /// removes what was extracted.
    Extract {
        archive: PathBuf,
        selection: Selection,
        dest: PathBuf,
        moving: bool,
    },
    /// Copy local `sources` into the archive folder `inner` (the one
    /// source under the name `rename_to`, when set); a move then deletes
    /// what was added.
    Import {
        archive: PathBuf,
        inner: PathBuf,
        sources: Vec<PathBuf>,
        rename_to: Option<OsString>,
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
    if let Some(remote) = &job.remote {
        // Archives are read with local tools: between one and a remote
        // panel nothing would be found (every file "failed").
        let local_sources = remote.src.is_none() && job.sources.iter().any(|s| inside(s).is_some());
        let local_dest = remote.dst.is_none() && split_archive_path(&job.destination).is_some();
        return (local_sources || local_dest).then(|| Err(t("vfs_action_unsupported")));
    }
    if job.operation == TransferOperation::Extract {
        return Some(extract_plan(job));
    }
    if !job.operation.uses_local_worker() {
        return None;
    }
    let (destination, rename_to) = target(job);
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
                selection: Selection {
                    base,
                    items: inner,
                    rename_to,
                },
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
                rename_to,
                moving,
            }))
        }
        _ => refused(),
    }
}

/// Extracting the whole archive (the job's one source) into its
/// destination folder.
fn extract_plan(job: &TransferJob) -> Result<ArchivePlan, String> {
    match job.sources.as_slice() {
        [archive] if crate::fs::archive::is_browsable(archive) => Ok(ArchivePlan::Extract {
            archive: archive.clone(),
            selection: Selection::whole(),
            dest: job.destination.clone(),
            moving: false,
        }),
        _ => Err(t("archive_unsupported_format")),
    }
}

/// The folder the job's items go into, and the new name of its one item
/// when the destination names the item itself (`<folder>/<other name>`,
/// as the transfer dialog lets the user type it). The pre-filled
/// `<folder>/<same name>` is that folder.
fn target(job: &TransferJob) -> (PathBuf, Option<OsString>) {
    let dest = &job.destination;
    let folder = (dest.clone(), None);
    let [single] = job.sources.as_slice() else {
        return folder;
    };
    if is_destination_parent_dir(&job.sources, dest, job.destination_kind, is_folder) {
        return folder;
    }
    let (Some(parent), Some(name)) = (dest.parent(), dest.file_name()) else {
        return folder;
    };
    let renamed = (Some(name) != single.file_name()).then(|| name.to_os_string());
    (parent.to_path_buf(), renamed)
}

/// `path` is a folder on disk, an archive file (the folder of its
/// entries) or a folder (or archive) inside an archive.
fn is_folder(path: &Path) -> bool {
    if path.is_dir() {
        return true;
    }
    match split_archive_path(path) {
        Some((_, inner)) if inner.as_os_str().is_empty() => true,
        Some((archive, _)) => {
            // An archive stored in the archive is a folder too (jobs then
            // refuse to reach into it).
            ArchiveVfs::open(archive).is_some_and(|vfs| {
                vfs.stat(path)
                    .is_ok_and(|e| e.is_dir || crate::fs::archive::is_browsable(path))
            })
        }
        None => false,
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
            selection,
            dest,
            moving,
        } => {
            let total = selection.items.len();
            let existing =
                extract_conflicts(&archive, &selection, &dest, &conflicts, &control).await?;
            run_archive_blocking(
                control,
                total,
                move |report| {
                    extract_selected(&archive, &selection, &dest, &existing, report)?;
                    if !moving {
                        return Ok(());
                    }
                    super::archive_move::remove_extracted(&archive, &selection, report)
                },
                "error_extraction_failed",
            )
            .await
        }
        ArchivePlan::Import {
            archive,
            inner,
            sources,
            rename_to,
            moving,
        } => {
            let mut edit = import_edit(&inner, &sources, rename_to.as_deref())
                .map_err(|e| anyhow!(e.to_string()))?;
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

/// Every local file and folder of `sources`, placed below `inner` (under
/// the name `rename_to`, for a single source). An unreadable folder fails
/// the import instead of leaving files out.
fn import_edit(
    inner: &Path,
    sources: &[PathBuf],
    rename_to: Option<&std::ffi::OsStr>,
) -> Result<ArchiveEdit, WalkError> {
    let mut add = Vec::new();
    for src in sources {
        let name = rename_to.map_or_else(
            || crate::fs::file_name_lossy(src),
            |n| n.to_string_lossy().into_owned(),
        );
        let top = inner.join(name);
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

/// Applies the job's conflict setting to the files `selection` takes that
/// already exist in `dest`.
async fn extract_conflicts(
    archive: &Path,
    selection: &Selection,
    dest: &Path,
    setup: &ConflictSetup,
    control: &JobControl,
) -> anyhow::Result<Existing> {
    let (clashes, placed) = {
        let (archive, selection, dest) =
            (archive.to_path_buf(), selection.clone(), dest.to_path_buf());
        tokio::task::spawn_blocking(move || clashing_files(&archive, &selection, &dest)).await??
    };
    let mut existing = Existing::default();
    if clashes.is_empty() {
        return Ok(existing);
    }
    let vfs = ArchiveVfs::open(archive.to_path_buf())
        .ok_or_else(|| anyhow!(t("vfs_action_unsupported")))?;
    let mut resolver = ConflictResolver {
        options: &setup.options,
        ctl: control,
        slot: &setup.slot,
        src_fs: &vfs,
        dst_fs: &LocalVfs,
        auto: None,
    };
    // Skips are reported by the extraction (a move keeps those entries).
    let mut ignored = TransferResults::default();
    // A new name must be free on disk and among the files extracted.
    let mut taken = placed;
    for (inner, rel) in clashes {
        let target = dest.join(&rel);
        let mut dst = target.clone();
        let action = resolver
            .resolve(&archive.join(&inner), &mut dst, &mut ignored)
            .await?;
        if !matches!(action, ConflictAction::Skip) && dst != target {
            dst = crate::fs::transfer::conflict::resolve_filename_conflict(&target, &|p| {
                p.exists() || p.strip_prefix(dest).is_ok_and(|r| taken.contains(r))
            });
            taken.insert(dst.strip_prefix(dest)?.to_path_buf());
        }
        match action {
            ConflictAction::Skip => {
                existing.skip.insert(rel);
            }
            ConflictAction::Proceed if dst == target => {
                existing.replace.insert(rel);
            }
            ConflictAction::Proceed => {
                let renamed = dst.strip_prefix(dest).map(Path::to_path_buf)?;
                existing.rename.insert(rel, renamed);
            }
        }
    }
    Ok(existing)
}

/// An entry's path in the archive and the path below the destination it
/// is extracted to.
type Placed = (PathBuf, PathBuf);

/// The file entries `selection` takes whose destination below `dest`
/// already holds something other than a folder, as (path in the archive,
/// path below `dest`); and every path below `dest` the extraction writes.
fn clashing_files(
    archive: &Path,
    selection: &Selection,
    dest: &Path,
) -> anyhow::Result<(Vec<Placed>, HashSet<PathBuf>)> {
    use crate::fs::archive::{format::EntryKind, safe_extract::ExtractGuard};
    let reader = crate::fs::archive::reader_of(archive)?;
    let mut placed = HashSet::new();
    let mut clashes = Vec::new();
    for entry in reader.entries(archive)? {
        let Some((inner, rel)) = ExtractGuard::sanitize(&entry.name)
            .ok()
            .and_then(|inner| selection.place(&inner).map(|rel| (inner, rel)))
        else {
            continue;
        };
        if !placed.insert(rel.clone()) || entry.kind == EntryKind::Dir {
            continue;
        }
        if std::fs::symlink_metadata(dest.join(&rel)).is_ok_and(|m| !m.is_dir()) {
            clashes.push((inner, rel));
        }
    }
    Ok((clashes, placed))
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
