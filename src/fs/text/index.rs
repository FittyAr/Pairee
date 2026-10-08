//! Sparse line index built incrementally in the background.
//!
//! Only the start of every [`STRIDE`]-th line is stored, so the index of a
//! multi-gigabyte file stays small; a line is found by jumping to its
//! checkpoint and splitting at most `STRIDE - 1` lines forward.

use super::lines::{LineFormat, for_each_line};
use super::store::ByteStore;
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

/// Lines between two stored checkpoints.
pub const STRIDE: u64 = 256;
/// Bytes read per step by the builder.
const BUILD_CHUNK: usize = 1024 * 1024;
/// Lines counted between two publications of the builder's progress.
const PUBLISH_EVERY: u64 = 16 * 1024;

/// What is known so far about the lines of a document.
#[derive(Debug, Default)]
pub struct LineIndex {
    /// Start offset of lines `0, STRIDE, 2 * STRIDE, ...`.
    checkpoints: Vec<u64>,
    /// Lines found so far.
    lines: u64,
    /// Bytes scanned so far (for progress).
    scanned: u64,
    /// The whole document has been scanned.
    complete: bool,
}

impl LineIndex {
    pub fn lines(&self) -> u64 {
        self.lines
    }

    pub fn scanned(&self) -> u64 {
        self.scanned
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Checkpoint offset at or before `line` and how many lines to skip from
    /// it; `None` while `line` has not been indexed yet.
    pub fn locate(&self, line: u64) -> Option<(u64, u64)> {
        if line >= self.lines {
            return None;
        }
        let offset = *self.checkpoints.get((line / STRIDE) as usize)?;
        Some((offset, line % STRIDE))
    }
}

pub type SharedIndex = Arc<RwLock<LineIndex>>;

/// Reads `index` even if a builder panicked while holding the lock.
pub fn read(index: &SharedIndex) -> std::sync::RwLockReadGuard<'_, LineIndex> {
    index.read().unwrap_or_else(|e| e.into_inner())
}

/// Background work that fills a [`SharedIndex`]; run it on a worker thread.
pub struct IndexJob {
    pub(super) store: Arc<dyn ByteStore>,
    pub(super) format: LineFormat,
    pub(super) start: u64,
    pub(super) index: SharedIndex,
    pub(super) cancel: Arc<AtomicBool>,
}

impl IndexJob {
    /// Scans the whole store, publishing progress every [`PUBLISH_EVERY`]
    /// lines. Returns early (index left incomplete) when cancelled.
    pub fn run(self) {
        let mut fresh: Vec<u64> = Vec::new();
        let mut lines = 0u64;
        let mut scanned = self.start;
        let result = for_each_line(
            self.store.as_ref(),
            self.format,
            self.start,
            BUILD_CHUNK,
            |start, _| {
                if self.cancel.load(Ordering::Relaxed) {
                    return ControlFlow::Break(());
                }
                if lines.is_multiple_of(STRIDE) {
                    fresh.push(start);
                }
                lines += 1;
                scanned = start;
                if lines.is_multiple_of(PUBLISH_EVERY) {
                    self.publish(&mut fresh, lines, scanned, false);
                }
                ControlFlow::Continue(())
            },
        );
        if let Err(e) = result {
            log::warn!("viewer line index stopped: {e}");
        }
        if !self.cancel.load(Ordering::Relaxed) {
            self.publish(&mut fresh, lines, self.store.len(), true);
        }
    }

    fn publish(&self, fresh: &mut Vec<u64>, lines: u64, scanned: u64, complete: bool) {
        let mut index = self.index.write().unwrap_or_else(|e| e.into_inner());
        index.checkpoints.append(fresh);
        index.lines = lines;
        index.scanned = scanned;
        index.complete = complete;
    }
}
