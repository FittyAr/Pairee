//! Debounce and burst coalescing of directory changes (pure; the caller
//! passes the clock so tests can drive time).

use super::monitor::{DirChange, WatchOrigin};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// When a folder with pending changes is due for a refresh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoalesceTiming {
    /// Quiet period after the last change.
    pub quiet: Duration,
    /// Longest delay after the first change of a burst that never goes
    /// quiet (e.g. a large copy), so the panel still updates now and then.
    pub max_wait: Duration,
}

impl Default for CoalesceTiming {
    fn default() -> Self {
        const QUIET_MS: u64 = 250;
        const MAX_WAIT_MS: u64 = 2_000;
        Self {
            quiet: Duration::from_millis(QUIET_MS),
            max_wait: Duration::from_millis(MAX_WAIT_MS),
        }
    }
}

#[derive(Debug)]
struct Pending {
    first: Instant,
    last: Instant,
    entries: BTreeSet<PathBuf>,
}

/// Collects changes per folder and releases one [`DirChange`] per folder
/// once it went quiet (or waited too long).
#[derive(Debug, Default)]
pub struct Coalescer {
    timing: CoalesceTiming,
    pending: HashMap<(WatchOrigin, PathBuf), Pending>,
}

impl Coalescer {
    pub fn new(timing: CoalesceTiming) -> Self {
        Self {
            timing,
            pending: HashMap::new(),
        }
    }

    /// Adds a change seen at `now`.
    pub fn record(&mut self, change: DirChange, now: Instant) {
        let key = (change.origin, change.dir);
        let pending = self.pending.entry(key).or_insert_with(|| Pending {
            first: now,
            last: now,
            entries: BTreeSet::new(),
        });
        pending.last = now;
        pending.entries.extend(change.entries);
    }

    /// Removes and returns the folders due at `now`, with the changed
    /// entries merged (sorted, without duplicates).
    pub fn take_due(&mut self, now: Instant) -> Vec<DirChange> {
        let timing = self.timing;
        let due: Vec<(WatchOrigin, PathBuf)> = self
            .pending
            .iter()
            .filter(|(_, p)| {
                now.saturating_duration_since(p.last) >= timing.quiet
                    || now.saturating_duration_since(p.first) >= timing.max_wait
            })
            .map(|(key, _)| key.clone())
            .collect();
        due.into_iter()
            .filter_map(|key| {
                let pending = self.pending.remove(&key)?;
                let (origin, dir) = key;
                Some(DirChange {
                    dir,
                    entries: pending.entries.into_iter().collect(),
                    origin,
                })
            })
            .collect()
    }

    /// Drops pending changes of folders for which `keep` is false.
    pub fn retain(&mut self, keep: impl Fn(WatchOrigin, &Path) -> bool) {
        self.pending.retain(|(origin, dir), _| keep(*origin, dir));
    }
}
