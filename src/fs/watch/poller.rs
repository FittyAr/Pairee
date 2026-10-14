//! Polling fallback: compares a cheap folder signature on an interval.

use super::monitor::{ChangeSink, DirChange};
use super::strategy::ChangeStrategy;
use crate::fs::vfs::Vfs;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, SystemTime};

/// Cheap fingerprint of a folder: its modification time and how many
/// entries it holds (names are not read beyond counting).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirSignature {
    pub modified: Option<SystemTime>,
    pub entries: usize,
}

impl DirSignature {
    /// `None` when the folder cannot be read (gone, no permission, …).
    pub fn read(dir: &Path) -> Option<Self> {
        let modified = std::fs::metadata(dir).ok()?.modified().ok();
        let entries = std::fs::read_dir(dir).ok()?.count();
        Some(Self { modified, entries })
    }

    /// The signature of `dir` on another filesystem (an SFTP server).
    pub fn read_vfs(vfs: &dyn Vfs, dir: &Path) -> Option<Self> {
        let modified = vfs.stat(dir).ok()?.modified;
        let entries = vfs.list(dir).ok()?.len();
        Some(Self { modified, entries })
    }

    /// The signature of the whole tree below `dir`: the newest time and
    /// the number of entries, over at most [`MAX_TREE_ENTRIES`] entries
    /// (a bigger tree is only checked that far).
    pub fn read_tree(vfs: &dyn Vfs, dir: &Path) -> Option<Self> {
        use crate::fs::walk::{LinkPolicy, WalkOptions, Walker};
        let mut signature = Self {
            modified: vfs.stat(dir).ok()?.modified,
            entries: 0,
        };
        let walker = Walker::new(vfs, dir, WalkOptions::with_links(LinkPolicy::Keep));
        for found in walker.flatten().take(MAX_TREE_ENTRIES) {
            signature.entries += 1;
            signature.modified = signature.modified.max(found.entry.modified);
        }
        Some(signature)
    }
}

/// Most entries a recursive poll looks at.
pub const MAX_TREE_ENTRIES: usize = 20_000;

pub struct PollStrategy {
    interval: Duration,
    /// Filesystem of the folder; `None` is the local disk.
    vfs: Option<Arc<dyn Vfs>>,
    /// Compare the whole tree, not only the folder.
    recursive: bool,
}

impl PollStrategy {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            vfs: None,
            recursive: false,
        }
    }

    /// Polls the whole tree below the folder.
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Polls a folder of `vfs`.
    pub fn over(vfs: Arc<dyn Vfs>, interval: Duration) -> Self {
        Self {
            interval,
            vfs: Some(vfs),
            recursive: false,
        }
    }

    fn signature(&self, dir: &Path) -> Option<DirSignature> {
        match (&self.vfs, self.recursive) {
            (Some(vfs), true) => DirSignature::read_tree(vfs.as_ref(), dir),
            (None, true) => DirSignature::read_tree(&crate::fs::vfs::LocalVfs, dir),
            (Some(vfs), false) => DirSignature::read_vfs(vfs.as_ref(), dir),
            (None, false) => DirSignature::read(dir),
        }
    }
}

impl ChangeStrategy for PollStrategy {
    fn run(&self, dir: &Path, stop: &Receiver<()>, sink: &ChangeSink) -> Result<(), String> {
        let mut last = self.signature(dir);
        sink.armed_at(dir, last.and_then(|s| s.modified));
        loop {
            match stop.recv_timeout(self.interval) {
                Err(RecvTimeoutError::Timeout) => {}
                _ => return Ok(()),
            }
            let now = self.signature(dir);
            if now != last {
                last = now;
                sink.report(DirChange::whole(dir.to_path_buf()));
            }
        }
    }
}
