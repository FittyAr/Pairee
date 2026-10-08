//! Recursive comparison of two directory trees (no UI, no async): reports
//! progress and checks for cancellation through a [`ScanObserver`].

use super::model::{DiffKind, SyncItem};
use super::options::{SyncFilter, SyncOptions};
use super::plan::apply_direction;
use crate::fs::compare::{
    EntryPair, FileSummary, ScannedDir, ScannedEntry, metadata_equal, mtime_within, pair_entries,
    scan_directory,
};
use std::path::{Path, PathBuf};

/// Scan statistics published while comparing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanProgress {
    pub dirs: usize,
    pub files: usize,
    /// Relative path of the folder being read.
    pub current: PathBuf,
}

/// Receives progress and is asked whether to stop.
pub trait ScanObserver {
    fn is_cancelled(&self) -> bool;
    fn progress(&self, progress: &ScanProgress);
}

/// Why a comparison stopped.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("cancelled")]
    Cancelled,
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Compares `left` and `right` recursively and returns every compared path
/// (folder pairs included, as context) with the direction's default action.
pub fn diff_trees(
    left: &Path,
    right: &Path,
    options: &SyncOptions,
    observer: &dyn ScanObserver,
) -> Result<Vec<SyncItem>, SyncError> {
    let mut walker = Walker {
        options,
        filter: options.filter(),
        observer,
        progress: ScanProgress::default(),
        items: Vec::new(),
    };
    walker.walk(Path::new(""), left, right)?;
    let mut items = walker.items;
    apply_direction(&mut items, options.direction);
    Ok(items)
}

struct Walker<'a> {
    options: &'a SyncOptions,
    filter: SyncFilter,
    observer: &'a dyn ScanObserver,
    progress: ScanProgress,
    items: Vec<SyncItem>,
}

impl Walker<'_> {
    /// Compares one folder pair; returns whether anything below differs.
    fn walk(&mut self, rel: &Path, left: &Path, right: &Path) -> Result<bool, SyncError> {
        self.enter(rel)?;
        let pairs = pair_entries(self.scan(left)?, self.scan(right)?);
        let mut differs = false;
        for pair in pairs {
            let item_rel = rel.join(pair.name());
            let (left_path, right_path) = match &pair {
                EntryPair::Left(l) => (l.path.clone(), right.join(&l.name)),
                EntryPair::Right(r) => (left.join(&r.name), r.path.clone()),
                EntryPair::Both(l, r) => (l.path.clone(), r.path.clone()),
            };
            let item = self.compare(pair, item_rel, left_path, right_path)?;
            differs |= item.kind != DiffKind::Equal;
            self.items.push(item);
        }
        Ok(differs)
    }

    fn enter(&mut self, rel: &Path) -> Result<(), SyncError> {
        if self.observer.is_cancelled() {
            return Err(SyncError::Cancelled);
        }
        self.progress.dirs += 1;
        self.progress.current = rel.to_path_buf();
        self.observer.progress(&self.progress);
        Ok(())
    }

    fn scan(&mut self, dir: &Path) -> Result<ScannedDir, SyncError> {
        let filter = &self.filter;
        let ci = self.options.compare.case_insensitive;
        let map = scan_directory(dir, ci, |e, meta| filter.accepts(e, meta))
            .map_err(|source| io_or_cancel(dir, source))?;
        self.progress.files += map.values().filter(|e| !e.summary.is_dir).count();
        Ok(map)
    }

    fn compare(
        &mut self,
        pair: EntryPair,
        rel_path: PathBuf,
        left_path: PathBuf,
        right_path: PathBuf,
    ) -> Result<SyncItem, SyncError> {
        let (left, right, kind) = match &pair {
            EntryPair::Left(l) => (Some(l.summary), None, DiffKind::OnlyLeft),
            EntryPair::Right(r) => (None, Some(r.summary), DiffKind::OnlyRight),
            EntryPair::Both(l, r) => (
                Some(l.summary),
                Some(r.summary),
                self.both(&rel_path, l, r)?,
            ),
        };
        Ok(SyncItem {
            left_bytes: self.side_bytes(&pair, true)?,
            right_bytes: self.side_bytes(&pair, false)?,
            rel_path,
            left_path,
            right_path,
            left,
            right,
            kind,
            action: super::model::SyncAction::Skip,
        })
    }

    /// Kind of a path present on both sides (recursing into folder pairs).
    fn both(
        &mut self,
        rel: &Path,
        l: &ScannedEntry,
        r: &ScannedEntry,
    ) -> Result<DiffKind, SyncError> {
        let (ls, rs) = (&l.summary, &r.summary);
        if ls.is_dir != rs.is_dir {
            return Ok(DiffKind::TypeMismatch);
        }
        if ls.is_dir {
            let differs = !(l.is_symlink || r.is_symlink) && self.walk(rel, &l.path, &r.path)?;
            return Ok(if differs {
                DiffKind::Differs
            } else {
                DiffKind::Equal
            });
        }
        if self.files_equal(l, r)? {
            return Ok(DiffKind::Equal);
        }
        Ok(newer_side(ls, rs, self.options.compare.mtime_tolerance))
    }

    fn files_equal(&self, l: &ScannedEntry, r: &ScannedEntry) -> Result<bool, SyncError> {
        let tolerance = self.options.compare.mtime_tolerance;
        match self.options.content_hash {
            Some(algorithm) if l.summary.size == r.summary.size => {
                let cancelled = || self.observer.is_cancelled();
                let hash = |e: &ScannedEntry| {
                    crate::fs::transfer::hash::hash_file(&e.path, algorithm, &cancelled)
                        .map_err(|source| io_or_cancel(&e.path, source))
                };
                Ok(hash(l)? == hash(r)?)
            }
            _ => Ok(metadata_equal(&l.summary, &r.summary, tolerance)),
        }
    }

    /// Size of one side of `pair` (a folder's files counted recursively).
    fn side_bytes(&self, pair: &EntryPair, left: bool) -> Result<u64, SyncError> {
        let entry = match (pair, left) {
            (EntryPair::Left(e) | EntryPair::Both(e, _), true) => e,
            (EntryPair::Right(e) | EntryPair::Both(_, e), false) => e,
            _ => return Ok(0),
        };
        if !entry.summary.is_dir {
            return Ok(entry.summary.size);
        }
        if matches!(pair, EntryPair::Both(..)) || entry.is_symlink {
            return Ok(0);
        }
        self.tree_bytes(&entry.path)
    }

    fn tree_bytes(&self, dir: &Path) -> Result<u64, SyncError> {
        if self.observer.is_cancelled() {
            return Err(SyncError::Cancelled);
        }
        let ci = self.options.compare.case_insensitive;
        let entries = scan_directory(dir, ci, |e, meta| self.filter.accepts(e, meta))
            .map_err(|source| io_or_cancel(dir, source))?;
        entries.values().try_fold(0u64, |acc, e| {
            let bytes = match (e.summary.is_dir, e.is_symlink) {
                (true, false) => self.tree_bytes(&e.path)?,
                (true, true) => 0,
                (false, _) => e.summary.size,
            };
            Ok(acc + bytes)
        })
    }
}

/// Which side of two different files is newer (equal times: [`DiffKind::Differs`]).
fn newer_side(l: &FileSummary, r: &FileSummary, tolerance: std::time::Duration) -> DiffKind {
    if mtime_within(l.modified, r.modified, tolerance) {
        return DiffKind::Differs;
    }
    match (l.modified, r.modified) {
        (Some(lt), Some(rt)) if lt > rt => DiffKind::LeftNewer,
        (Some(_), Some(_)) | (None, Some(_)) => DiffKind::RightNewer,
        _ => DiffKind::LeftNewer,
    }
}

fn io_or_cancel(path: &Path, source: std::io::Error) -> SyncError {
    if source.kind() == std::io::ErrorKind::Interrupted {
        SyncError::Cancelled
    } else {
        SyncError::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}
