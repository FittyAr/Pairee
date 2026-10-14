//! Jobs that run one at a time, in the order they were started, each one
//! delivering its result: for work that cannot be interrupted half-way
//! (remote changes, renames), where superseding a running job would lose
//! what it did.

use super::JobSlot;
use std::collections::VecDeque;

type QueuedJob<T> = Box<dyn FnOnce() -> T + Send>;

/// A [`JobSlot`] plus the jobs waiting for it.
pub struct JobQueue<T> {
    running: JobSlot<T>,
    queued: VecDeque<QueuedJob<T>>,
}

impl<T> Default for JobQueue<T> {
    fn default() -> Self {
        Self {
            running: JobSlot::default(),
            queued: VecDeque::new(),
        }
    }
}

impl<T> std::fmt::Debug for JobQueue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobQueue")
            .field("running", &self.running)
            .field("queued", &self.queued.len())
            .finish()
    }
}

impl<T: Send + 'static> JobQueue<T> {
    /// Runs `job` after the jobs started before it.
    pub fn push(&mut self, job: impl FnOnce() -> T + Send + 'static) {
        self.queued.push_back(Box::new(job));
        self.start_next();
    }

    /// A job is running or waiting.
    pub fn is_running(&self) -> bool {
        self.running.is_running() || !self.queued.is_empty()
    }

    /// The result of the job that finished, if one did; the next job then
    /// starts.
    pub fn poll(&mut self) -> Option<T> {
        let result = self.running.poll();
        self.start_next();
        result
    }

    fn start_next(&mut self) {
        if self.running.is_running() {
            return;
        }
        if let Some(job) = self.queued.pop_front() {
            self.running.start(move |_| job());
        }
    }
}
