//! Find file (Alt+F7) as a [`JobSlot`] job: the scanner's matches are
//! collected in the background and published as progress, so the results
//! list fills while the search runs and a new search or Esc cancels it.

use super::{JobContext, JobSlot};
use crate::fs::search::SearchQuery;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// A match: its path and whether it is a folder.
pub type SearchHit = (PathBuf, bool);

/// Most matches a search lists.
pub const MAX_RESULTS: usize = 500;

/// How often the matches found so far are published.
const REPORT_EVERY: Duration = Duration::from_millis(100);

/// The running search: its result and progress are the matches found.
pub type SearchSlot = JobSlot<Vec<SearchHit>, Vec<SearchHit>>;

/// Starts `query` in `slot`, cancelling the previous search.
pub fn start(slot: &mut SearchSlot, query: SearchQuery) {
    slot.start(move |ctx| collect(query, ctx));
}

fn collect(query: SearchQuery, ctx: &JobContext<Vec<SearchHit>>) -> Vec<SearchHit> {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return Vec::new();
    };
    // Dropping the receiver stops the scanner at its next match.
    let mut rx = {
        let _runtime = handle.enter();
        crate::fs::search::find_files(query)
    };
    let mut hits = Vec::new();
    let mut reported = Instant::now();
    while !ctx.is_cancelled() && hits.len() < MAX_RESULTS {
        match handle.block_on(tokio::time::timeout(REPORT_EVERY, rx.recv())) {
            Ok(Some(hit)) => hits.push(hit),
            Ok(None) => break,
            Err(_) => {}
        }
        if reported.elapsed() >= REPORT_EVERY {
            reported = Instant::now();
            ctx.report(hits.clone());
            super::wake_event_loop();
        }
    }
    hits
}
