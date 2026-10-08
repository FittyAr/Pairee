//! Hand-off of the user's answer to a "file exists" conflict from the UI to
//! the waiting transfer worker, without polling.

use super::conflict::ConflictResolution;
use crate::lock::LockExt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

/// One pending conflict answer per job. The worker [`ConflictSlot::wait`]s;
/// the UI [`ConflictSlot::answer`]s; cancelling the job [`ConflictSlot::wake`]s it.
#[derive(Debug, Default)]
pub struct ConflictSlot {
    answer: Mutex<Option<ConflictResolution>>,
    notify: Notify,
}

impl ConflictSlot {
    /// Clears any stale answer before a new question is asked.
    pub fn reset(&self) {
        *self.answer.lock_safe() = None;
    }

    /// Stores the user's choice and wakes the worker.
    pub fn answer(&self, resolution: ConflictResolution) {
        *self.answer.lock_safe() = Some(resolution);
        self.notify.notify_one();
    }

    /// Wakes the worker without an answer (e.g. the job was cancelled).
    pub fn wake(&self) {
        self.notify.notify_one();
    }

    /// Waits for an answer; `None` if `cancelled` became true first.
    pub async fn wait(&self, cancelled: &AtomicBool) -> Option<ConflictResolution> {
        loop {
            // Register interest before checking, so a concurrent answer
            // between the check and the await is never lost.
            let notified = self.notify.notified();
            if let Some(answer) = *self.answer.lock_safe() {
                return Some(answer);
            }
            if cancelled.load(Ordering::Relaxed) {
                return None;
            }
            notified.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn answer_wakes_waiter() {
        let slot = Arc::new(ConflictSlot::default());
        let waiter = {
            let slot = Arc::clone(&slot);
            tokio::spawn(async move { slot.wait(&AtomicBool::new(false)).await })
        };
        tokio::time::sleep(Duration::from_millis(10)).await;
        slot.answer(ConflictResolution::Skip);
        let got = tokio::time::timeout(Duration::from_secs(5), waiter)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got, Some(ConflictResolution::Skip));
    }

    #[tokio::test]
    async fn cancellation_wakes_waiter_without_answer() {
        let slot = Arc::new(ConflictSlot::default());
        let cancelled = Arc::new(AtomicBool::new(false));
        let waiter = {
            let (slot, cancelled) = (Arc::clone(&slot), Arc::clone(&cancelled));
            tokio::spawn(async move { slot.wait(&cancelled).await })
        };
        tokio::time::sleep(Duration::from_millis(10)).await;
        cancelled.store(true, Ordering::SeqCst);
        slot.wake();
        let got = tokio::time::timeout(Duration::from_secs(5), waiter)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn reset_clears_previous_answer() {
        let slot = ConflictSlot::default();
        slot.answer(ConflictResolution::OverwriteAll);
        slot.reset();
        assert!(slot.answer.lock_safe().is_none());
    }
}
