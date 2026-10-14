//! Folders reopened from the last session are not checked on the UI thread:
//! one on a network share that is offline would freeze start-up while the
//! system waits for it. The tab opens on the saved folder at once and its
//! (background) listing finds out: when the folder cannot be read, the job
//! lists its nearest readable ancestor (or home) instead
//! (`listing::ListingRequest::fallback`). Meanwhile the panel title says
//! "connecting", then "unavailable" once [`CONNECT_TIMEOUT`] passed or
//! nothing could be read.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How long a restored folder may take before the title says it is
/// unavailable (the listing keeps waiting for it).
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// A restored folder whose first listing has not arrived yet (or failed).
#[derive(Debug, Clone)]
pub struct RestoreCheck {
    path: PathBuf,
    started: Instant,
    failed: bool,
}

/// What the panel title says about a restored folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreStatus {
    Connecting,
    Unavailable,
}

impl RestoreCheck {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            started: Instant::now(),
            failed: false,
        }
    }

    /// The listing of `listed` (asked for `asked`) arrived: `true` when it
    /// answers this check, which then ends unless `failed`.
    pub fn answered_by(&mut self, asked: &Path, failed: bool) -> bool {
        if asked != self.path {
            return false;
        }
        self.failed = failed;
        true
    }

    pub fn failed(&self) -> bool {
        self.failed
    }

    /// Whether `path` is the restored folder being checked.
    pub fn covers(&self, path: &Path) -> bool {
        path == self.path
    }

    /// The status shown while the panel still shows `current`.
    pub fn status(&self, current: &Path, now: Instant) -> Option<RestoreStatus> {
        if current != self.path {
            return None;
        }
        let late = now.saturating_duration_since(self.started) >= CONNECT_TIMEOUT;
        Some(if self.failed || late {
            RestoreStatus::Unavailable
        } else {
            RestoreStatus::Connecting
        })
    }

    #[cfg(test)]
    pub fn started_at(mut self, started: Instant) -> Self {
        self.started = started;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connecting_turns_unavailable_after_the_timeout_or_a_failure() {
        let dir = PathBuf::from("/share/docs");
        let now = Instant::now();
        let check = RestoreCheck::new(dir.clone()).started_at(now);
        assert_eq!(check.status(&dir, now), Some(RestoreStatus::Connecting));
        let late = now + CONNECT_TIMEOUT;
        assert_eq!(check.status(&dir, late), Some(RestoreStatus::Unavailable));
        assert_eq!(check.status(Path::new("/elsewhere"), now), None);
        let mut check = RestoreCheck::new(dir.clone()).started_at(now);
        assert!(!check.answered_by(Path::new("/other"), true));
        assert!(check.answered_by(&dir, true));
        assert_eq!(check.status(&dir, now), Some(RestoreStatus::Unavailable));
    }
}
