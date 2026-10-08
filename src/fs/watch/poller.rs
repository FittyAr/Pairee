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
}

pub struct PollStrategy {
    interval: Duration,
    /// Filesystem of the folder; `None` is the local disk.
    vfs: Option<Arc<dyn Vfs>>,
}

impl PollStrategy {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            vfs: None,
        }
    }

    /// Polls a folder of `vfs`.
    pub fn over(vfs: Arc<dyn Vfs>, interval: Duration) -> Self {
        Self {
            interval,
            vfs: Some(vfs),
        }
    }

    fn signature(&self, dir: &Path) -> Option<DirSignature> {
        match &self.vfs {
            Some(vfs) => DirSignature::read_vfs(vfs.as_ref(), dir),
            None => DirSignature::read(dir),
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
