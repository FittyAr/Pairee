//! Cooperative cancellation flag shared between the UI and a background job.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Cheap, clonable cancellation flag. Jobs poll [`CancelToken::is_cancelled`]
/// at convenient points (between files, inside network progress callbacks).
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation. Idempotent.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}
