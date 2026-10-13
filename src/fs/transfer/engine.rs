//! The transfer engine: a queue and one coordinator task that runs the
//! queued jobs one after another. The coordinator sleeps until the queue
//! changes (no polling); each job runs in its own task, so a panicking
//! backend fails that job instead of the coordinator.

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::backend;
use super::events::{EventSender, TransferEvent};
use super::job::{TransferJob, TransferJobStatus};
use super::queue::TransferQueue;

pub struct TransferEngine {
    pub queue: TransferQueue,
    event_tx: EventSender,
    coordinator: Option<JoinHandle<()>>,
}

impl TransferEngine {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<TransferEvent>) {
        let (event_tx, event_rx) = EventSender::channel();
        let engine = Self {
            queue: TransferQueue::new(),
            event_tx,
            coordinator: None,
        };
        (engine, event_rx)
    }

    /// Queues `job`; it starts when the jobs before it are done.
    pub fn submit_job(&mut self, job: TransferJob) {
        self.queue.enqueue(job);
        if self.coordinator.is_none() {
            let (queue, event_tx) = (self.queue.clone(), self.event_tx.clone());
            self.coordinator = Some(tokio::spawn(coordinate(queue, event_tx)));
        }
    }
}

/// Runs queued jobs in order, waiting for the queue to change when none is
/// left.
async fn coordinate(queue: TransferQueue, event_tx: EventSender) {
    loop {
        match queue.dequeue() {
            Some(job) => run_to_end(&queue, &event_tx, job).await,
            None => queue.changed().await,
        }
    }
}

/// Runs `job` in its own task and waits for it; a task that ends without a
/// final status (a panic) fails the job.
async fn run_to_end(queue: &TransferQueue, event_tx: &EventSender, job: TransferJob) {
    let job_id = job.id;
    let worker = tokio::spawn(run_job(queue.clone(), event_tx.clone(), job));
    if worker.await.is_ok() {
        return;
    }
    let unfinished = queue.get(job_id).is_some_and(|j| !j.is_terminal());
    if !unfinished {
        return;
    }
    log::warn!("Transfer worker for job {job_id} terminated unexpectedly (likely a panic)");
    let error = crate::config::localization::t("transfer_worker_crashed");
    queue.update_job(job_id, |j| {
        j.status = TransferJobStatus::Failed;
        j.log_lines.push(error.clone());
    });
    let _ = event_tx.send(TransferEvent::JobFailed { job_id, error });
}

/// Runs the backend of `job` and records its final status.
async fn run_job(queue: TransferQueue, event_tx: EventSender, job: TransferJob) {
    let job_id = job.id;
    queue.update_job(job_id, |j| j.status = TransferJobStatus::Scanning);
    let cancelled = std::sync::Arc::clone(&job.is_cancelled);
    match backend::run_job(job, event_tx.clone()).await {
        Ok(results) => queue.update_job(job_id, |j| {
            j.status = TransferJobStatus::Completed;
            j.results = results;
        }),
        Err(e) => {
            let is_cancel = cancelled.load(std::sync::atomic::Ordering::Relaxed);
            queue.update_job(job_id, |j| {
                j.status = if is_cancel {
                    TransferJobStatus::Cancelled
                } else {
                    TransferJobStatus::Failed
                };
            });
            let error = if is_cancel {
                crate::config::localization::t("transfer_job_cancelled")
            } else {
                e.to_string()
            };
            let _ = event_tx.send(TransferEvent::JobFailed { job_id, error });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::transfer::job::TransferOperation;
    use crate::fs::transfer::options::TransferOptions;
    use std::time::Duration;

    /// Jobs queued one after another all run, in order, without polling.
    #[tokio::test]
    async fn queued_jobs_run_one_after_another() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        std::fs::write(&src, b"a").unwrap();
        let (mut engine, mut rx) = TransferEngine::new();
        let ids: Vec<_> = (0..3)
            .map(|i| {
                let out = dir.path().join(format!("out{i}"));
                std::fs::create_dir(&out).unwrap();
                let job = TransferJob::new(
                    TransferOperation::Copy,
                    vec![src.clone()],
                    out,
                    TransferOptions::default(),
                );
                let id = job.id;
                engine.submit_job(job);
                id
            })
            .collect();
        let mut completed = Vec::new();
        while completed.len() < ids.len() {
            let event = tokio::time::timeout(Duration::from_secs(10), rx.recv())
                .await
                .expect("jobs finish")
                .expect("engine alive");
            if let TransferEvent::JobCompleted { job_id, .. } = event {
                completed.push(job_id);
            }
        }
        assert_eq!(completed, ids);
        for i in 0..3 {
            assert!(dir.path().join(format!("out{i}")).join("a.txt").exists());
        }
    }
}
