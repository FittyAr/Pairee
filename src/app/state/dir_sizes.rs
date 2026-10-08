//! Folder sizes shown in a panel's size column (Space / F3 on a folder,
//! "calculate folder sizes").
//!
//! Requests are computed one batch at a time in a background [`JobSlot`];
//! folders requested while a batch runs wait in a queue. Results stay until
//! the panel lists another directory.

use crate::app::jobs::JobSlot;
use crate::fs::du::{DirSize, LocalSource, ScanControl, scan};
use crate::fs::ssh::SharedSshClient;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Progress of a running batch: the folder being measured and its bytes so far.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirSizeProgress {
    pub target: PathBuf,
    pub bytes: u64,
}

type Batch = Vec<(PathBuf, DirSize)>;

#[derive(Debug, Default)]
pub struct DirSizes {
    known: HashMap<PathBuf, DirSize>,
    /// Folders waiting for the running batch to finish.
    queue: Vec<PathBuf>,
    /// Folders of the running batch.
    running: Vec<PathBuf>,
    ssh: Option<SharedSshClient>,
    job: JobSlot<Batch, DirSizeProgress>,
}

impl DirSizes {
    /// Computed size of `path`, if any.
    pub fn get(&self, path: &Path) -> Option<&DirSize> {
        self.known.get(path)
    }

    pub fn is_running(&self) -> bool {
        self.job.is_running()
    }

    pub fn progress(&self) -> Option<DirSizeProgress> {
        self.job.progress()
    }

    /// Queues `paths` (folders of a panel connected through `ssh`, if any).
    /// Folders already known or already requested are skipped.
    pub fn request(&mut self, paths: Vec<PathBuf>, ssh: Option<SharedSshClient>) {
        self.ssh = ssh;
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
        let ssh = self.ssh.clone();
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
                let node = match &ssh {
                    Some(client) => scan(client, &path, false, &ctl),
                    None => scan(&LocalSource, &path, false, &ctl),
                };
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
        self.known.extend(batch);
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

    /// Keeps only the results whose folder is still listed.
    pub fn retain_listed(&mut self, listed: &std::collections::HashSet<PathBuf>) {
        self.known.retain(|path, _| listed.contains(path));
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
        sizes.request(vec![folder.clone()], None);
        assert!(sizes.poll());
        assert_eq!(sizes.get(&folder).unwrap().bytes, 42);
        assert!(!sizes.is_running());
    }

    #[test]
    fn known_folders_are_not_recomputed() {
        let dir = tree();
        let folder = dir.path().join("a");
        let mut sizes = DirSizes::default();
        sizes.request(vec![folder.clone()], None);
        sizes.poll();
        sizes.request(vec![folder.clone()], None);
        assert!(!sizes.is_running());
        assert!(!sizes.poll());
    }

    #[test]
    fn clear_and_retain() {
        let dir = tree();
        let folder = dir.path().join("a");
        let mut sizes = DirSizes::default();
        sizes.request(vec![folder.clone()], None);
        sizes.poll();
        sizes.retain_listed(&[folder.clone()].into_iter().collect());
        assert!(sizes.get(&folder).is_some());
        sizes.retain_listed(&Default::default());
        assert!(sizes.get(&folder).is_none());
        sizes.request(vec![folder.clone()], None);
        sizes.poll();
        sizes.clear();
        assert!(sizes.get(&folder).is_none());
    }
}
