//! Polling fallback: compares a cheap folder signature on an interval.

use super::monitor::{ChangeSink, DirChange};
use super::strategy::ChangeStrategy;
use std::path::Path;
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
}

pub struct PollStrategy {
    interval: Duration,
}

impl PollStrategy {
    pub fn new(interval: Duration) -> Self {
        Self { interval }
    }
}

impl ChangeStrategy for PollStrategy {
    fn run(&self, dir: &Path, stop: &Receiver<()>, sink: &ChangeSink) -> Result<(), String> {
        let mut last = DirSignature::read(dir);
        loop {
            match stop.recv_timeout(self.interval) {
                Err(RecvTimeoutError::Timeout) => {}
                _ => return Ok(()),
            }
            let now = DirSignature::read(dir);
            if now != last {
                last = now;
                sink.report(DirChange::whole(dir.to_path_buf()));
            }
        }
    }
}
