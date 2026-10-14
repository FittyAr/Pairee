//! Options a running job takes from the transfer panel while it runs.
//!
//! A worker copies its [`TransferOptions`] when it starts, so most edits in
//! the panel's Options tab only apply to jobs that have not started. These
//! can change on the fly: the job and its worker share one [`LiveOptions`]
//! (the panel writes changes to it, the worker reads it):
//!
//! * the speed limit, re-read while a file is being copied;
//! * "Verify after copy" and the conflict policy, from the next file on.

use super::options::TransferOptions;
use std::sync::Mutex;

/// Changes made to a job's options after it was queued; `None` keeps the
/// value it started with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LiveChanges {
    /// Bytes per second (`Some(None)`: no limit).
    pub limit_bandwidth_rate: Option<Option<u64>>,
    pub verify_after_copy: Option<bool>,
    pub conflict_resolution: Option<String>,
}

/// The shared handle (see the module docs).
#[derive(Debug, Default)]
pub struct LiveOptions {
    changes: Mutex<LiveChanges>,
}

impl LiveOptions {
    /// Records the live-adjustable values of `options` (after an edit).
    pub fn publish(&self, options: &TransferOptions) {
        *self.lock() = LiveChanges {
            limit_bandwidth_rate: Some(options.limit_bandwidth_rate),
            verify_after_copy: Some(options.verify_after_copy),
            conflict_resolution: Some(options.conflict_resolution.clone()),
        };
    }

    /// Applies the changes to `options` (before each file).
    pub fn apply_to(&self, options: &mut TransferOptions) {
        let changes = self.lock().clone();
        if let Some(rate) = changes.limit_bandwidth_rate {
            options.limit_bandwidth_rate = rate;
        }
        if let Some(verify) = changes.verify_after_copy {
            options.verify_after_copy = verify;
        }
        if let Some(policy) = changes.conflict_resolution {
            options.conflict_resolution = policy;
        }
    }

    /// The speed limit now, when it was changed (`Some(None)`: removed).
    pub fn bandwidth_rate(&self) -> Option<Option<u64>> {
        self.lock().limit_bandwidth_rate
    }

    /// The conflict policy now, when it was changed.
    pub fn conflict_resolution(&self) -> Option<String> {
        self.lock().conflict_resolution.clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, LiveChanges> {
        self.changes.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_published_values_override_the_start_options() {
        let live = LiveOptions::default();
        let mut options = TransferOptions {
            conflict_resolution: "skip".into(),
            ..TransferOptions::default()
        };
        live.apply_to(&mut options);
        assert_eq!(options.conflict_resolution, "skip", "nothing changed yet");
        let edited = TransferOptions {
            limit_bandwidth_rate: Some(42),
            verify_after_copy: true,
            conflict_resolution: "overwrite".into(),
            ..TransferOptions::default()
        };
        live.publish(&edited);
        live.apply_to(&mut options);
        assert_eq!(options.limit_bandwidth_rate, Some(42));
        assert!(options.verify_after_copy);
        assert_eq!(options.conflict_resolution, "overwrite");
    }
}
