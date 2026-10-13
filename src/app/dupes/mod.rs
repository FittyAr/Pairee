//! State of the duplicate finder: the background search
//! ([`crate::fs::dupes::find_duplicates`]), its report, the modification
//! times read with it and the files marked for deletion. Files a finished
//! delete job removed leave the report without a new search.

#[cfg(test)]
mod tests;

use crate::app::jobs::JobSlot;
use crate::fs::dupes::keep::{Keep, all_but};
use crate::fs::dupes::{
    DupeControl, DupeGroup, DupeOptions, DupeProgress, DupeReport, find_duplicates,
};
use crate::fs::transfer::job::{TransferJob, TransferOperation, TransferResults};
use crate::fs::vfs::PanelSource;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What the background search hands back.
#[derive(Debug, Default)]
struct DupeResult {
    report: DupeReport,
    modified: HashMap<PathBuf, SystemTime>,
}

#[derive(Debug, Default)]
pub struct DupesState {
    source: PanelSource,
    job: JobSlot<DupeResult, DupeProgress>,
    result: Option<DupeResult>,
    /// Files marked for deletion.
    marked: HashSet<PathBuf>,
    /// Highlighted file (index over every group's files).
    pub cursor: usize,
}

impl DupesState {
    /// Searches `roots` on `source` (replacing any earlier result).
    pub fn start(&mut self, roots: Vec<PathBuf>, source: PanelSource, options: DupeOptions) {
        self.result = None;
        self.marked.clear();
        self.cursor = 0;
        self.source = source;
        let vfs = self.source.vfs();
        self.job.start(move |ctx| {
            let cancelled = || ctx.is_cancelled();
            let report = |p: &DupeProgress| ctx.report(p.clone());
            let ctl = DupeControl {
                cancelled: &cancelled,
                progress: &report,
            };
            let report = find_duplicates(vfs.as_ref(), &roots, &options, &ctl);
            let modified = report
                .groups
                .iter()
                .flat_map(|g| &g.files)
                .filter_map(|f| Some((f.clone(), vfs.metadata(f).ok()?.modified?)))
                .collect();
            DupeResult { report, modified }
        });
        self.poll();
    }

    /// Installs a finished search. Returns `true` when the view changed (or
    /// a search is still running and its progress moved).
    pub fn poll(&mut self) -> bool {
        if let Some(result) = self.job.poll() {
            self.result = Some(result);
            return true;
        }
        self.job.is_running()
    }

    pub fn cancel(&mut self) {
        self.job.cancel();
    }

    pub fn is_scanning(&self) -> bool {
        self.job.is_running()
    }

    pub fn progress(&self) -> Option<DupeProgress> {
        self.job.progress()
    }

    pub fn report(&self) -> Option<&DupeReport> {
        self.result.as_ref().map(|r| &r.report)
    }

    pub fn groups(&self) -> &[DupeGroup] {
        self.report().map_or(&[], |r| r.groups.as_slice())
    }

    /// Every file of every group, in display order.
    pub fn files(&self) -> impl Iterator<Item = &PathBuf> {
        self.groups().iter().flat_map(|g| &g.files)
    }

    pub fn file_count(&self) -> usize {
        self.groups().iter().map(|g| g.files.len()).sum()
    }

    /// The highlighted file.
    pub fn selected(&self) -> Option<&PathBuf> {
        self.files().nth(self.cursor)
    }

    pub fn modified(&self, path: &Path) -> Option<SystemTime> {
        self.result.as_ref()?.modified.get(path).copied()
    }

    pub fn is_marked(&self, path: &Path) -> bool {
        self.marked.contains(path)
    }

    /// The marked files, in display order.
    pub fn marked(&self) -> Vec<PathBuf> {
        self.files()
            .filter(|f| self.marked.contains(*f))
            .cloned()
            .collect()
    }

    /// Bytes the marked files take.
    pub fn marked_bytes(&self) -> u64 {
        self.groups()
            .iter()
            .map(|g| g.size * g.files.iter().filter(|f| self.marked.contains(*f)).count() as u64)
            .sum()
    }

    /// Marks or unmarks the highlighted file.
    pub fn toggle_selected(&mut self) {
        let Some(path) = self.selected().cloned() else {
            return;
        };
        if !self.marked.remove(&path) {
            self.marked.insert(path);
        }
    }

    /// Marks every file but the one `keep` picks in each group.
    pub fn mark_all_but(&mut self, keep: Keep) {
        let marked: Vec<PathBuf> = self
            .groups()
            .iter()
            .flat_map(|g| all_but(&g.files, keep, |f| self.modified(f)))
            .collect();
        self.marked = marked.into_iter().collect();
    }

    pub fn unmark_all(&mut self) {
        self.marked.clear();
    }

    /// Drops what a finished delete `job` removed (same source only).
    /// Returns `true` if anything changed.
    pub fn job_finished(&mut self, job: &TransferJob, results: &TransferResults) -> bool {
        if job.operation != TransferOperation::Delete || !job.ran_on(&self.source) {
            return false;
        }
        let deleted: Vec<&Path> = results
            .completed_files
            .iter()
            .map(|f| f.src.as_path())
            .collect();
        self.forget(&deleted)
    }

    /// Removes `deleted` files from the report and the marks.
    pub fn forget(&mut self, deleted: &[&Path]) -> bool {
        let Some(result) = self.result.as_mut() else {
            return false;
        };
        let changed = result.report.forget(deleted.iter().copied());
        for path in deleted {
            self.marked.remove(*path);
        }
        let len = self.file_count();
        self.cursor = self.cursor.min(len.saturating_sub(1));
        changed
    }
}
