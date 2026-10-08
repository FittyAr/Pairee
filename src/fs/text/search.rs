//! Incremental, cancellable text search over a document's lines.

use super::encoding::decode_line;
use super::lines::{LineFormat, for_each_line};
use super::store::ByteStore;
use encoding_rs::Encoding;
use std::ops::ControlFlow;
use std::sync::Arc;

/// Bytes read per step while searching.
pub const SEARCH_CHUNK: usize = 1024 * 1024;

/// Substring test with optional case folding.
#[derive(Debug, Clone)]
pub struct Matcher {
    needle: String,
    case_sensitive: bool,
}

impl Matcher {
    pub fn new(query: &str, case_sensitive: bool) -> Self {
        let needle = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };
        Self {
            needle,
            case_sensitive,
        }
    }

    pub fn matches(&self, line: &str) -> bool {
        if self.case_sensitive {
            line.contains(&self.needle)
        } else {
            line.to_lowercase().contains(&self.needle)
        }
    }
}

/// Where a search starts and wraps.
#[derive(Debug, Clone, Copy)]
pub struct SearchFrom {
    /// First line searched and its byte offset.
    pub line: u64,
    pub offset: u64,
    /// Offset of line 0 (after the byte-order mark), where the search wraps.
    pub top: u64,
}

/// A search to run on a worker thread (see [`SearchJob::run`]).
pub struct SearchJob {
    pub(super) store: Arc<dyn ByteStore>,
    pub(super) format: LineFormat,
    pub(super) encoding: &'static Encoding,
    pub(super) from: SearchFrom,
    pub(super) matcher: Matcher,
}

impl SearchJob {
    /// First matching line at or after the start, wrapping to the top once.
    /// `cancelled` is polled per line; `progress` receives 0–100.
    pub fn run(&self, cancelled: &dyn Fn() -> bool, progress: &dyn Fn(u8)) -> Option<u64> {
        let total = self.store.len().max(1);
        let SearchFrom { line, offset, top } = self.from;
        let below = total.saturating_sub(offset);
        // Bytes already covered before the wrap, for a monotonic percentage.
        let report = |covered: u64| progress((covered.min(total) * 100 / total) as u8);
        if let Some(hit) = self.scan(offset, line, None, cancelled, &|at| report(at - offset)) {
            return hit;
        }
        self.scan(top, 0, Some(line), cancelled, &|at| {
            report(below + at - top)
        })
        .flatten()
    }

    /// Scans from `offset` (line number `first`) up to line `stop`. Returns
    /// `Some(Some(line))` on a match, `Some(None)` when cancelled or stopped,
    /// `None` when the end was reached without a match.
    fn scan(
        &self,
        offset: u64,
        first: u64,
        stop: Option<u64>,
        cancelled: &dyn Fn() -> bool,
        progress: &dyn Fn(u64),
    ) -> Option<Option<u64>> {
        let mut line = first;
        let mut outcome = None;
        let result = for_each_line(
            self.store.as_ref(),
            self.format,
            offset,
            SEARCH_CHUNK,
            |start, content| {
                if cancelled() || stop == Some(line) {
                    outcome = Some(None);
                    return ControlFlow::Break(());
                }
                if self.matcher.matches(&decode_line(self.encoding, content)) {
                    outcome = Some(Some(line));
                    return ControlFlow::Break(());
                }
                if line.is_multiple_of(1024) {
                    progress(start);
                }
                line += 1;
                ControlFlow::Continue(())
            },
        );
        if result.is_err() {
            return Some(None);
        }
        outcome
    }
}
