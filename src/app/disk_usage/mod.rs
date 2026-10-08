//! State of the disk usage view (ncdu-like): one background scan builds the
//! whole size tree of a folder, then navigation in and out of subfolders is
//! instant. The tree is kept (cached) until a rescan or a scan of another
//! folder; items a finished delete job removed from the scanned source
//! (local disk, SFTP server, zip archive) leave the tree without a rescan.

#[cfg(test)]
mod tests;

use crate::app::jobs::JobSlot;
use crate::fs::du::{DuNode, ScanControl, ScanProgress, scan};
use crate::fs::transfer::job::{TransferJob, TransferOperation, TransferResults};
use crate::fs::vfs::PanelSource;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct DiskUsageState {
    /// Folder the tree was (or is being) scanned from.
    root: PathBuf,
    source: PanelSource,
    tree: Option<DuNode>,
    job: JobSlot<DuNode, ScanProgress>,
    /// Names from `root` down to the folder on screen.
    trail: Vec<String>,
    /// Highlighted row in the folder on screen.
    pub cursor: usize,
}

impl DiskUsageState {
    /// Shows `root`, reusing the cached tree when it is the same local folder.
    pub fn open(&mut self, root: PathBuf, source: PanelSource) {
        let cached = self.tree.is_some() && self.source.is_local() && source.is_local();
        if cached && self.root == root {
            return;
        }
        self.root = root;
        self.source = source;
        self.trail.clear();
        self.cursor = 0;
        self.rescan();
    }

    /// Scans the root again (the folder on screen is kept when it still exists).
    pub fn rescan(&mut self) {
        self.tree = None;
        let root = self.root.clone();
        let vfs = self.source.vfs();
        self.job.start(move |ctx| {
            let cancelled = || ctx.is_cancelled();
            let report = |p: &ScanProgress| ctx.report(p.clone());
            let ctl = ScanControl {
                cancelled: &cancelled,
                progress: &report,
            };
            scan(vfs.as_ref(), &root, true, &ctl)
        });
        self.poll();
    }

    /// Installs a finished scan. Returns `true` when the view changed (or a
    /// scan is still running and its progress moved).
    pub fn poll(&mut self) -> bool {
        if let Some(tree) = self.job.poll() {
            self.tree = Some(tree);
            while self
                .tree
                .as_ref()
                .and_then(|t| t.descend(&self.trail))
                .is_none()
            {
                self.trail.pop();
            }
            self.clamp_cursor();
            return true;
        }
        self.job.is_running()
    }

    /// Stops a running scan.
    pub fn cancel(&mut self) {
        self.job.cancel();
    }

    pub fn is_scanning(&self) -> bool {
        self.job.is_running()
    }

    pub fn progress(&self) -> Option<ScanProgress> {
        self.job.progress()
    }

    /// Whole tree (for the grand total).
    pub fn tree(&self) -> Option<&DuNode> {
        self.tree.as_ref()
    }

    /// The folder on screen.
    pub fn current(&self) -> Option<&DuNode> {
        self.tree.as_ref()?.descend(&self.trail)
    }

    /// Path of the folder on screen.
    pub fn current_path(&self) -> PathBuf {
        self.trail.iter().fold(self.root.clone(), |p, n| p.join(n))
    }

    /// The highlighted item.
    pub fn selected(&self) -> Option<&DuNode> {
        self.current()?.children.get(self.cursor)
    }

    /// Path of the highlighted item.
    pub fn selected_path(&self) -> Option<PathBuf> {
        let name = &self.selected()?.name;
        Some(self.current_path().join(name))
    }

    /// Opens the highlighted folder. Returns `false` on a file.
    pub fn enter(&mut self) -> bool {
        let Some(name) = self.selected().filter(|n| n.is_dir).map(|n| n.name.clone()) else {
            return false;
        };
        self.trail.push(name);
        self.cursor = 0;
        true
    }

    /// Back to the parent folder, highlighting the folder we came from.
    /// Returns `false` at the scanned root.
    pub fn leave(&mut self) -> bool {
        let Some(name) = self.trail.pop() else {
            return false;
        };
        self.cursor = self
            .current()
            .and_then(|dir| dir.children.iter().position(|c| c.name == name))
            .unwrap_or(0);
        true
    }

    /// Drops what a finished delete `job` removed from the scanned source
    /// (no rescan, no polling of the server). Returns `true` if any.
    pub fn job_finished(&mut self, job: &TransferJob, results: &TransferResults) -> bool {
        let remote = job
            .ssh
            .as_ref()
            .and_then(|e| e.src.as_ref().or(e.dst.as_ref()));
        let same_source = match (self.source.ssh(), remote) {
            (Some(shown), Some(job)) => shown.is_same_server(job),
            (None, None) => true,
            _ => false,
        };
        if job.operation != TransferOperation::Delete || !same_source {
            return false;
        }
        self.remove_deleted(results.completed_files.iter().map(|f| f.src.as_path()))
    }

    /// Removes `paths` (deleted items) from the tree. Returns `true` if any.
    pub fn remove_deleted<'a>(&mut self, paths: impl IntoIterator<Item = &'a Path>) -> bool {
        let mut changed = false;
        for path in paths {
            changed |= self.remove_from_tree(path);
        }
        if changed {
            self.clamp_cursor();
        }
        changed
    }

    fn remove_from_tree(&mut self, path: &Path) -> bool {
        let Some(tree) = self.tree.as_mut() else {
            return false;
        };
        let Ok(rel) = path.strip_prefix(&self.root) else {
            return false;
        };
        let names: Vec<String> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        tree.remove(&names).is_some()
    }

    fn clamp_cursor(&mut self) {
        let len = self.current().map_or(0, |dir| dir.children.len());
        self.cursor = self.cursor.min(len.saturating_sub(1));
    }
}
