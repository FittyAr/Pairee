//! Folder sizes shown in a panel's size column (Space / F3 on a folder,
//! "calculate folder sizes").
//!
//! Requests are computed one batch at a time in a background [`JobSlot`];
//! folders requested while a batch runs wait in a queue. Results stay until
//! the panel lists another directory; every reread of the same directory
//! (manual, after an operation, or an automatic refresh) measures again the
//! folders whose modification time changed since they were measured.

use crate::app::jobs::JobSlot;
use crate::fs::FileEntry;
use crate::fs::du::{DirSize, ScanControl, scan};
use crate::fs::vfs::PanelSource;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Progress of a running batch: the folder being measured and its bytes so far.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirSizeProgress {
    pub target: PathBuf,
    pub bytes: u64,
}

/// A measured folder and the modification time the listing showed for it.
#[derive(Debug, Clone)]
struct Measured {
    size: DirSize,
    modified: Option<SystemTime>,
}

type Batch = Vec<(PathBuf, DirSize)>;

#[derive(Debug, Default)]
pub struct DirSizes {
    known: HashMap<PathBuf, Measured>,
    /// Modification times of the folders in the latest listing. Results are
    /// stamped with them: listings are what later rereads compare against
    /// (Windows updates folder times in listings lazily).
    listed: HashMap<PathBuf, Option<SystemTime>>,
    /// Folders waiting for the running batch to finish.
    queue: Vec<PathBuf>,
    /// Folders of the running batch.
    running: Vec<PathBuf>,
    source: PanelSource,
    job: JobSlot<Batch, DirSizeProgress>,
}

impl DirSizes {
    /// Computed size of `path`, if any.
    pub fn get(&self, path: &Path) -> Option<&DirSize> {
        self.known.get(path).map(|m| &m.size)
    }

    pub fn is_running(&self) -> bool {
        self.job.is_running()
    }

    pub fn progress(&self) -> Option<DirSizeProgress> {
        self.job.progress()
    }

    /// Queues `paths` (folders of a panel showing `source`). Folders
    /// already known or already requested are skipped.
    pub fn request(&mut self, paths: Vec<PathBuf>, source: PanelSource) {
        self.source = source;
        for path in paths {
            let pending = self.running.contains(&path) || self.queue.contains(&path);
            if !pending && !self.known.contains_key(&path) {
                self.queue.push(path);
            }
        }
        if !self.job.is_running() {
            self.start_next();
        }
    }

    fn start_next(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        self.running = std::mem::take(&mut self.queue);
        let paths = self.running.clone();
        let vfs = self.source.vfs();
        self.job.start(move |ctx| {
            let mut sizes = Vec::with_capacity(paths.len());
            for path in paths {
                if ctx.is_cancelled() {
                    break;
                }
                let report = |p: &crate::fs::du::ScanProgress| {
                    ctx.report(DirSizeProgress {
                        target: path.clone(),
                        bytes: p.bytes,
                    })
                };
                let cancelled = || ctx.is_cancelled();
                let ctl = ScanControl {
                    cancelled: &cancelled,
                    progress: &report,
                };
                let node = scan(vfs.as_ref(), &path, false, &ctl);
                sizes.push((path, node.size));
            }
            sizes
        });
    }

    /// Stores a finished batch and starts the next one. Returns `true` when
    /// something changed.
    pub fn poll(&mut self) -> bool {
        let Some(batch) = self.job.poll() else {
            return false;
        };
        self.running.clear();
        for (path, size) in batch {
            let modified = self.listed.get(&path).copied().flatten();
            self.known.insert(path, Measured { size, modified });
        }
        self.start_next();
        true
    }

    /// Stops the running batch and drops the queue. Returns `true` when
    /// something was running.
    pub fn cancel(&mut self) -> bool {
        let was_running = self.job.is_running();
        self.job.cancel();
        self.queue.clear();
        self.running.clear();
        was_running
    }

    /// Forgets every result (another directory was listed).
    pub fn clear(&mut self) {
        self.cancel();
        self.known.clear();
    }

    /// Forgets the results of `paths` (a watcher reported them changed)
    /// and measures those folders again in the background.
    pub fn invalidate(&mut self, paths: &[PathBuf]) {
        let stale = paths
            .iter()
            .filter(|path| self.known.remove(*path).is_some())
            .cloned()
            .collect();
        self.remeasure(stale);
    }

    /// Applies a new listing of the same directory: results of folders no
    /// longer listed are dropped, and folders whose modification time
    /// changed are measured again in the background.
    pub fn sync_listing(&mut self, entries: &[FileEntry]) {
        let listed: HashMap<&Path, &FileEntry> =
            entries.iter().map(|e| (e.path.as_path(), e)).collect();
        let mut stale = Vec::new();
        self.known
            .retain(|path, measured| match listed.get(path.as_path()) {
                None => false,
                // A link lists its target's time; its size is the link's.
                Some(entry) if !entry.is_symlink && entry.modified != measured.modified => {
                    stale.push(path.clone());
                    false
                }
                Some(_) => true,
            });
        self.listed = entries
            .iter()
            .filter(|e| e.is_dir)
            .map(|e| (e.path.clone(), e.modified))
            .collect();
        self.remeasure(stale);
    }

    /// Measures `stale` folders (results already dropped) again.
    fn remeasure(&mut self, stale: Vec<PathBuf>) {
        if !stale.is_empty() {
            let source = self.source.clone();
            self.request(stale, source);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a")).unwrap();
        std::fs::write(dir.path().join("a/f"), [0u8; 42]).unwrap();
        dir
    }

    #[test]
    fn request_computes_inline_without_runtime() {
        let dir = tree();
        let folder = dir.path().join("a");
        let mut sizes = DirSizes::default();
        sizes.request(vec![folder.clone()], Default::default());
        assert!(sizes.poll());
        assert_eq!(sizes.get(&folder).unwrap().bytes, 42);
        assert!(!sizes.is_running());
    }

    #[test]
    fn known_folders_are_not_recomputed() {
        let dir = tree();
        let folder = dir.path().join("a");
        let mut sizes = DirSizes::default();
        sizes.request(vec![folder.clone()], Default::default());
        sizes.poll();
        sizes.request(vec![folder.clone()], Default::default());
        assert!(!sizes.is_running());
        assert!(!sizes.poll());
    }

    /// The entry of `path` as a listing of its parent shows it.
    fn listed(path: &Path) -> FileEntry {
        crate::fs::vfs::Vfs::stat(&crate::fs::vfs::LocalVfs, path)
            .unwrap()
            .into_file_entry()
    }

    #[test]
    fn rereads_measure_folders_whose_time_changed() {
        let dir = tree();
        let folder = dir.path().join("a");
        let mut sizes = DirSizes::default();
        sizes.sync_listing(&[listed(&folder)]);
        sizes.request(vec![folder.clone()], Default::default());
        sizes.poll();
        sizes.sync_listing(&[listed(&folder)]);
        assert!(!sizes.is_running(), "unchanged folder kept");
        std::fs::write(folder.join("g"), [0u8; 8]).unwrap();
        let mut entry = listed(&folder);
        // Coarse clocks may keep the time: make the change visible.
        entry.modified = entry
            .modified
            .map(|t| t + std::time::Duration::from_secs(5));
        sizes.sync_listing(&[entry]);
        assert!(sizes.poll());
        assert_eq!(sizes.get(&folder).unwrap().bytes, 50);
    }

    #[test]
    fn clear_and_retain() {
        let dir = tree();
        let folder = dir.path().join("a");
        let mut sizes = DirSizes::default();
        sizes.sync_listing(&[listed(&folder)]);
        sizes.request(vec![folder.clone()], Default::default());
        sizes.poll();
        sizes.sync_listing(&[listed(&folder)]);
        assert!(sizes.get(&folder).is_some());
        sizes.sync_listing(&[]);
        assert!(sizes.get(&folder).is_none());
        sizes.request(vec![folder.clone()], Default::default());
        sizes.poll();
        sizes.clear();
        assert!(sizes.get(&folder).is_none());
    }
}
