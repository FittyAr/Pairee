use std::sync::Arc;

use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::events::TransferEvent;
use super::job::{TransferJob, TransferJobStatus};
use super::queue::TransferQueue;
use super::worker::TransferWorker;

pub struct TransferEngine {
    pub queue: TransferQueue,
    event_tx: mpsc::UnboundedSender<TransferEvent>,
    active_coordinator_handle: Option<JoinHandle<()>>,
}

impl TransferEngine {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<TransferEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let engine = Self {
            queue: TransferQueue::new(),
            event_tx,
            active_coordinator_handle: None,
        };
        (engine, event_rx)
    }

    pub fn submit_job(&mut self, job: TransferJob) {
        self.queue.enqueue(job);
        self.trigger_processing_loop();
    }

    pub fn trigger_processing_loop(&mut self) {
        if self.active_coordinator_handle.is_some() {
            return;
        }

        let queue = self.queue.clone();
        let event_tx = self.event_tx.clone();

        let handle = tokio::spawn(async move {
            let mut active_worker: Option<(uuid::Uuid, JoinHandle<()>)> = None;

            loop {
                // Verificar si el trabajador activo ha terminado (o entrado en pánico/abortado)
                if let Some((job_id, ref worker_handle)) = active_worker {
                    if worker_handle.is_finished() {
                        let jobs = queue.get_all();
                        if let Some(job) = jobs.iter().find(|j| j.id == job_id) {
                            if !job.is_terminal() {
                                // The worker task exited without producing a
                                // terminal status, which almost always means
                                // it panicked. We do not have the panic
                                // payload because the handle was already
                                // detached, so we log a warning that includes
                                // the job id so the operator can correlate
                                // with a core file or backtrace. The user
                                // sees the generic message in the UI.
                                log::warn!(
                                    "Transfer worker for job {} terminated unexpectedly \
                                     (likely a panic); marking job as failed",
                                    job_id
                                );
                                queue.update_job(job_id, |j| {
                                    j.status = TransferJobStatus::Failed;
                                    j.log_lines.push(
                                        "Error: Worker task terminated unexpectedly.".to_string(),
                                    );
                                });
                                let _ = event_tx.send(TransferEvent::JobFailed {
                                    job_id,
                                    error: "Worker task terminated unexpectedly".to_string(),
                                });
                            }
                        }
                        active_worker = None;
                    }
                }

                let jobs = queue.get_all();

                // 1. Verificar si hay algún trabajo activo
                let any_running = active_worker.is_some()
                    || jobs.iter().any(|j| {
                        matches!(
                            j.status,
                            TransferJobStatus::Scanning
                                | TransferJobStatus::Transferring
                                | TransferJobStatus::Verifying
                        )
                    });

                if !any_running {
                    // Buscar el primer trabajo Queued en la cola
                    if let Some(job) = queue.dequeue() {
                        let job_id = job.id;
                        let queue_clone = queue.clone();
                        let event_tx_clone = event_tx.clone();

                        let worker_handle = tokio::spawn(async move {
                            let worker = TransferWorker::new(
                                job.id,
                                job.operation,
                                job.sources,
                                job.destination,
                                job.options.clone(),
                                Arc::clone(&job.is_paused),
                                Arc::clone(&job.is_cancelled),
                                Arc::clone(&job.skip_file_flag),
                                event_tx_clone.clone(),
                                job.active_conflict.clone(),
                            );

                            queue_clone.update_job(job_id, |j| {
                                j.status = TransferJobStatus::Scanning;
                            });
                            let _ = event_tx_clone.send(TransferEvent::ScanStarted { job_id });

                            match worker.run().await {
                                Ok(results) => {
                                    queue_clone.update_job(job_id, |j| {
                                        j.status = TransferJobStatus::Completed;
                                        j.results = results.clone();
                                    });
                                    let _ = event_tx_clone
                                        .send(TransferEvent::JobCompleted { job_id, results });
                                }
                                Err(e) => {
                                    let err_msg = e.to_string();
                                    // Cancellation must be detected from the
                                    // job's atomic flag, not from the error
                                    // message string. A future refactor that
                                    // changes the wording of the "cancelled"
                                    // error would otherwise silently flip
                                    // user-cancelled jobs into "Failed".
                                    let is_cancel = queue_clone
                                        .get_all()
                                        .iter()
                                        .find(|j| j.id == job_id)
                                        .map(|j| j.is_cancelled.load(std::sync::atomic::Ordering::Relaxed))
                                        .unwrap_or(false);
                                    queue_clone.update_job(job_id, |j| {
                                        j.status = if is_cancel {
                                            TransferJobStatus::Cancelled
                                        } else {
                                            TransferJobStatus::Failed
                                        };
                                    });
                                    let _ = event_tx_clone.send(TransferEvent::JobFailed {
                                        job_id,
                                        error: if is_cancel {
                                            "Job cancelled by user".to_string()
                                        } else {
                                            err_msg
                                        },
                                    });
                                }
                            }
                        });

                        active_worker = Some((job_id, worker_handle));
                    }
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });

        self.active_coordinator_handle = Some(handle);
    }
}
