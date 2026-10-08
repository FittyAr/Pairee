//! Reusable background-job facility.
//!
//! Every piece of blocking I/O that used to run on the UI/event-loop thread
//! (directory listings, SFTP calls, git status/network operations, quick-view
//! loads, …) goes through a [`JobSlot`]:
//!
//! * [`JobSlot::start`] runs a closure on Tokio's blocking pool and tags the
//!   result with a **generation** number. Starting a new job in the same slot
//!   cancels the previous one and bumps the generation, so a late result from
//!   an outdated request is silently dropped by [`JobSlot::poll`].
//! * The closure receives a [`JobContext`] carrying a [`CancelToken`] (checked
//!   cooperatively, e.g. from git `transfer_progress` callbacks) and a
//!   `watch`-based progress reporter: the UI only ever sees the latest value,
//!   so a fast producer cannot flood the event loop.
//! * The event loop drains slots once per tick (`poll`) and marks the UI dirty
//!   when something arrived — the same pull model the transfer engine uses.
//!
//! Outside a Tokio runtime (unit tests, early start-up) jobs run inline so the
//! behaviour stays deterministic.

mod cancel;
mod slot;

#[cfg(test)]
mod tests;

pub use slot::{JobContext, JobSlot};

/// Runs `f` on Tokio's blocking pool, or inline when no runtime is available.
pub fn spawn_blocking_or_inline<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            handle.spawn_blocking(f);
        }
        Err(_) => f(),
    }
}

static JOB_FINISHED: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// Wakes the event loop so it drains background results now (job results,
/// folder change notifications, …).
pub fn wake_event_loop() {
    JOB_FINISHED.notify_one();
}

/// Resolves when a job finished (or [`wake_event_loop`] was called) since
/// the last call, so the event loop can apply results immediately instead
/// of waiting for the next input tick.
pub async fn finished() {
    JOB_FINISHED.notified().await;
}
