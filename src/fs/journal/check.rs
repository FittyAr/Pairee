//! Preconditions of a command about to run (an undo or a redo): every entry
//! it reads must still be the one the journal recorded and every place it
//! writes to must be free. Entries that fail are skipped, never forced.

use super::command::{FileBatch, FileStep, FsCommand};
use crate::config::localization::t;
use crate::fs::LinkKind;
use crate::fs::multi_rename::{Step, TargetFs};
use crate::fs::stamp::{Stamp, entry_exists};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Why an entry is left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// The entry to act on is gone.
    Missing,
    /// The entry was modified since the operation.
    Changed,
    /// Something else now exists where the entry would go.
    Occupied,
    /// The copy overwrote a file: deleting it would lose data.
    Replaced,
    /// The folder is no longer empty.
    NotEmpty,
    /// The original of a copy is gone: the copy is the only one left.
    OriginalMissing,
}

impl SkipReason {
    pub fn text(self) -> String {
        t(match self {
            Self::Missing => "journal_skip_missing",
            Self::Changed => "journal_skip_changed",
            Self::Occupied => "journal_skip_occupied",
            Self::Replaced => "journal_skip_replaced",
            Self::NotEmpty => "journal_skip_not_empty",
            Self::OriginalMissing => "journal_skip_original_missing",
        })
    }
}

/// An entry left out and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub path: PathBuf,
    pub reason: SkipReason,
}

/// Outcome of [`check`]: what can still run and what is skipped.
#[derive(Debug, Clone)]
pub struct Checked {
    /// The command restricted to the entries that passed (`None`: nothing).
    pub runnable: Option<FsCommand>,
    pub skipped: Vec<Skipped>,
}

/// Checks `command` against the filesystem as it is now.
pub fn check(command: &FsCommand) -> Checked {
    let mut skipped = Vec::new();
    let runnable = match command {
        // The SFTP executor re-checks every target and rolls back on error.
        FsCommand::Rename { ssh: Some(_), .. } => Some(command.clone()),
        FsCommand::Rename { steps, ssh: None } => {
            check_renames(steps, &mut skipped).then(|| command.clone())
        }
        FsCommand::Move(batch) => {
            batch_of(batch, &mut skipped, check_transfer).map(FsCommand::Move)
        }
        FsCommand::Copy(batch) => {
            batch_of(batch, &mut skipped, check_transfer).map(FsCommand::Copy)
        }
        FsCommand::RemoveCopies(batch) => {
            batch_of(batch, &mut skipped, check_copy_removal).map(FsCommand::RemoveCopies)
        }
        FsCommand::Trash { paths } => paths_of(paths, &mut skipped, |p| {
            (!entry_exists(p)).then_some(SkipReason::Missing)
        })
        .map(|paths| FsCommand::Trash { paths }),
        FsCommand::Restore { paths } => paths_of(paths, &mut skipped, |p| {
            entry_exists(p).then_some(SkipReason::Occupied)
        })
        .map(|paths| FsCommand::Restore { paths }),
        FsCommand::MakeDir { path } => single(path, &mut skipped, occupied(path), command),
        FsCommand::RemoveDir { path } => single(path, &mut skipped, dir_removal(path), command),
        FsCommand::MakeFile { path } => single(path, &mut skipped, occupied(path), command),
        FsCommand::RemoveFile { path } => single(path, &mut skipped, file_removal(path), command),
        FsCommand::Link { link, target, .. } => {
            let reason = occupied(link).or((!entry_exists(target)).then_some(SkipReason::Missing));
            single(link, &mut skipped, reason, command)
        }
        FsCommand::Unlink { link, target, kind } => single(
            link,
            &mut skipped,
            link_removal(link, target, *kind),
            command,
        ),
        FsCommand::NotUndoable { .. } => None,
    };
    Checked { runnable, skipped }
}

fn occupied(path: &Path) -> Option<SkipReason> {
    entry_exists(path).then_some(SkipReason::Occupied)
}

/// `command` when `reason` is `None`, else records the skip of `path`.
fn single(
    path: &Path,
    skipped: &mut Vec<Skipped>,
    reason: Option<SkipReason>,
    command: &FsCommand,
) -> Option<FsCommand> {
    match reason {
        None => Some(command.clone()),
        Some(reason) => {
            skipped.push(Skipped {
                path: path.to_path_buf(),
                reason,
            });
            None
        }
    }
}

/// The paths that pass `reason_of` (which returns why one is skipped).
fn paths_of(
    paths: &[PathBuf],
    skipped: &mut Vec<Skipped>,
    reason_of: impl Fn(&Path) -> Option<SkipReason>,
) -> Option<Vec<PathBuf>> {
    let mut kept = Vec::new();
    for path in paths {
        match reason_of(path) {
            None => kept.push(path.clone()),
            Some(reason) => skipped.push(Skipped {
                path: path.clone(),
                reason,
            }),
        }
    }
    (!kept.is_empty()).then_some(kept)
}

/// The files of `batch` that pass `check_file` (which returns the path and
/// reason of a skip).
fn batch_of(
    batch: &FileBatch,
    skipped: &mut Vec<Skipped>,
    check_file: fn(&FileStep) -> Option<Skipped>,
) -> Option<FileBatch> {
    let mut files = Vec::new();
    for file in &batch.files {
        match check_file(file) {
            None => files.push(file.clone()),
            Some(skip) => skipped.push(skip),
        }
    }
    (!files.is_empty()).then(|| FileBatch {
        files,
        ..batch.clone()
    })
}

/// Reason a stamped entry no longer qualifies.
fn stamp_reason(path: &Path, stamp: Option<Stamp>) -> Option<SkipReason> {
    match stamp {
        _ if !entry_exists(path) => Some(SkipReason::Missing),
        Some(stamp) if !stamp.matches(path) => Some(SkipReason::Changed),
        _ => None,
    }
}

/// A move or copy: the source is unchanged and the destination free.
fn check_transfer(file: &FileStep) -> Option<Skipped> {
    if let Some(reason) = stamp_reason(&file.from, file.stamp) {
        return Some(skip(&file.from, reason));
    }
    occupied(&file.to).map(|reason| skip(&file.to, reason))
}

/// Deleting a copy: it is unchanged, did not overwrite anything and its
/// original still exists.
fn check_copy_removal(file: &FileStep) -> Option<Skipped> {
    let reason = if file.replaced {
        Some(SkipReason::Replaced)
    } else if let Some(reason) = stamp_reason(&file.to, file.stamp) {
        Some(reason)
    } else {
        (!entry_exists(&file.from)).then_some(SkipReason::OriginalMissing)
    };
    reason.map(|reason| skip(&file.to, reason))
}

fn skip(path: &Path, reason: SkipReason) -> Skipped {
    Skipped {
        path: path.to_path_buf(),
        reason,
    }
}

fn dir_removal(path: &Path) -> Option<SkipReason> {
    if !path.is_dir() {
        return Some(SkipReason::Missing);
    }
    let empty = std::fs::read_dir(path).is_ok_and(|mut entries| entries.next().is_none());
    (!empty).then_some(SkipReason::NotEmpty)
}

/// A created file is only removed while it is still empty.
fn file_removal(path: &Path) -> Option<SkipReason> {
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() && meta.len() == 0 => None,
        Ok(_) => Some(SkipReason::Changed),
        Err(_) => Some(SkipReason::Missing),
    }
}

fn link_removal(link: &Path, target: &Path, kind: LinkKind) -> Option<SkipReason> {
    let Ok(meta) = std::fs::symlink_metadata(link) else {
        return Some(SkipReason::Missing);
    };
    let intact = match kind {
        LinkKind::Symbolic => std::fs::read_link(link).is_ok_and(|t| t == target),
        LinkKind::Hard => meta.is_file(),
    };
    (!intact).then_some(SkipReason::Changed)
}

/// Replays local renames on a virtual view of the folder: each source must
/// exist and each target be free when its step runs. Renames are all or
/// nothing (a swap cannot run halfway), so the first failure stops.
fn check_renames(steps: &[Step], skipped: &mut Vec<Skipped>) -> bool {
    let fs = TargetFs::local();
    let mut gone: HashSet<String> = HashSet::new();
    let mut added: HashSet<String> = HashSet::new();
    let exists = |path: &Path, gone: &HashSet<String>, added: &HashSet<String>| {
        let key = fs.path_key(path);
        added.contains(&key) || (!gone.contains(&key) && entry_exists(path))
    };
    for step in steps {
        let (from, to) = (fs.path_key(&step.from), fs.path_key(&step.to));
        let reason = if !exists(&step.from, &gone, &added) {
            Some(skip(&step.from, SkipReason::Missing))
        } else if from != to && exists(&step.to, &gone, &added) {
            Some(skip(&step.to, SkipReason::Occupied))
        } else {
            None
        };
        if let Some(reason) = reason {
            skipped.push(reason);
            return false;
        }
        added.remove(&from);
        gone.insert(from);
        gone.remove(&to);
        added.insert(to);
    }
    true
}
