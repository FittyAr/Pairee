//! Generation-tagged single-flight job slot.

use super::cancel::CancelToken;
use std::panic::{AssertUnwindSafe, catch_unwind};
use tokio::sync::{mpsc, watch};

/// Handed to the job closure: cancellation flag plus a progress reporter.
pub struct JobContext<P> {
    cancel: CancelToken,
    progress: watch::Sender<Option<P>>,
}

impl<P> JobContext<P> {
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Clone of the cancellation flag (for callbacks that must be `'static`).
    pub fn cancel_token(&self) -> CancelToken {
        self.cancel.clone()
    }

    /// Publishes the latest progress value; older unread values are replaced.
    pub fn report(&self, progress: P) {
        // `send_replace` never fails, even when the receiver is gone.
        self.progress.send_replace(Some(progress));
    }
}

/// A job's outcome as sent over the channel: `None` means the job panicked.
type Delivery<T> = (u64, Option<T>);

/// Owns at most one *current* background job of type `T` (with progress `P`).
///
/// Starting a new job supersedes the previous one: it is cancelled and its
/// result, if it still arrives, is dropped because its generation is stale.
pub struct JobSlot<T, P = ()> {
    generation: u64,
    running: bool,
    tx: mpsc::UnboundedSender<Delivery<T>>,
    rx: mpsc::UnboundedReceiver<Delivery<T>>,
    cancel: CancelToken,
    progress: Option<watch::Receiver<Option<P>>>,
}

impl<T, P> std::fmt::Debug for JobSlot<T, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobSlot")
            .field("generation", &self.generation)
            .field("running", &self.running)
            .finish()
    }
}

impl<T, P> Default for JobSlot<T, P> {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            generation: 0,
            running: false,
            tx,
            rx,
            cancel: CancelToken::new(),
            progress: None,
        }
    }
}

impl<T, P> JobSlot<T, P>
where
    T: Send + 'static,
    P: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts `job` in the background, superseding any running job.
    /// Returns the new generation number.
    pub fn start<F>(&mut self, job: F) -> u64
    where
        F: FnOnce(&JobContext<P>) -> T + Send + 'static,
    {
        self.cancel.cancel();
        self.generation = self.generation.wrapping_add(1);
        self.running = true;
        let cancel = CancelToken::new();
        self.cancel = cancel.clone();
        let (progress_tx, progress_rx) = watch::channel(None);
        self.progress = Some(progress_rx);

        let generation = self.generation;
        let tx = self.tx.clone();
        let ctx = JobContext {
            cancel,
            progress: progress_tx,
        };
        super::spawn_blocking_or_inline(move || {
            let outcome = catch_unwind(AssertUnwindSafe(|| job(&ctx))).ok();
            if outcome.is_none() {
                log::error!("background job (generation {generation}) panicked");
            }
            let _ = tx.send((generation, outcome));
            super::notify_finished();
        });
        generation
    }

    /// Drains finished jobs. Returns the result of the *current* generation
    /// once it is available; results of superseded generations are dropped.
    pub fn poll(&mut self) -> Option<T> {
        let mut current = None;
        while let Ok((generation, outcome)) = self.rx.try_recv() {
            if generation != self.generation {
                continue;
            }
            self.running = false;
            self.progress = None;
            if outcome.is_some() {
                current = outcome;
            }
        }
        current
    }

    /// Cancels the current job and makes sure its result is ignored.
    pub fn cancel(&mut self) {
        if self.running {
            self.cancel.cancel();
            self.generation = self.generation.wrapping_add(1);
            self.running = false;
            self.progress = None;
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Latest progress value published by the current job, if any.
    pub fn progress(&self) -> Option<P>
    where
        P: Clone,
    {
        self.progress.as_ref().and_then(|rx| rx.borrow().clone())
    }
}
