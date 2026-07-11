use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use std::time::Duration;

use super::job::{TransferJob, TransferJobStatus};
use super::events::TransferEvent;
use super::queue::TransferQueue;
use super::worker::TransferWorker;

pub struct TransferEngine {
    pub queue: TransferQueue,
    event_tx: mpsc::UnboundedSender<TransferEvent>,
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
    skip_file_flag: Arc<AtomicBool>,
    active_worker_handle: Option<JoinHandle<()>>,
}

impl TransferEngine {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<TransferEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let engine = Self {
            queue: TransferQueue::new(),
            event_tx,
            is_paused: Arc::new(AtomicBool::new(false)),
            is_cancelled: Arc::new(AtomicBool::new(false)),
            skip_file_flag: Arc::new(AtomicBool::new(false)),
            active_worker_handle: None,
        };
        (engine, event_rx)
    }

    pub fn submit_job(&mut self, job: TransferJob) {
        self.queue.enqueue(job);
        self.trigger_processing_loop();
    }

    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
        self.queue.set_active_status(TransferJobStatus::Paused);
    }

    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
        self.queue.set_active_status(TransferJobStatus::Transferring);
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
        self.queue.cancel_all_pending();
    }

    pub fn skip_file(&self) {
        self.skip_file_flag.store(true, Ordering::SeqCst);
    }

    pub fn get_queue_snapshot(&self) -> Vec<TransferJob> {
        self.queue.get_all()
    }

    pub fn trigger_processing_loop(&mut self) {
        if let Some(ref handle) = self.active_worker_handle {
            if !handle.is_finished() {
                return;
            }
        }

        // Resetear flags antes de iniciar un nuevo bucle
        self.is_paused.store(false, Ordering::SeqCst);
        self.is_cancelled.store(false, Ordering::SeqCst);
        self.skip_file_flag.store(false, Ordering::SeqCst);

        let queue = self.queue.clone();
        let event_tx = self.event_tx.clone();
        let is_paused = Arc::clone(&self.is_paused);
        let is_cancelled = Arc::clone(&self.is_cancelled);
        let skip_file_flag = Arc::clone(&self.skip_file_flag);

        let handle = tokio::spawn(async move {
            loop {
                // Si la cola está cancelada globalmente, la limpiamos y salimos
                if is_cancelled.load(Ordering::Relaxed) {
                    queue.clear_active();
                    break;
                }

                // Intentar sacar el siguiente trabajo
                let job = match queue.dequeue() {
                    Some(j) => j,
                    None => {
                        // No hay más trabajos
                        queue.clear_active();
                        break;
                    }
                };

                // Resetear flags para este nuevo trabajo
                is_paused.store(false, Ordering::SeqCst);
                is_cancelled.store(false, Ordering::SeqCst);
                skip_file_flag.store(false, Ordering::SeqCst);

                // Configurar el worker
                let worker = TransferWorker::new(
                    job.id,
                    job.operation,
                    job.sources,
                    job.destination,
                    job.options,
                    Arc::clone(&is_paused),
                    Arc::clone(&is_cancelled),
                    Arc::clone(&skip_file_flag),
                    event_tx.clone(),
                    job.active_conflict.clone(),
                );

                // Correr el worker
                match worker.run().await {
                    Ok(results) => {
                        queue.update_active_results(|r| {
                            *r = results;
                        });
                        queue.set_active_status(TransferJobStatus::Completed);
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        if err_msg.contains("cancelled") {
                            queue.set_active_status(TransferJobStatus::Cancelled);
                            let _ = event_tx.send(TransferEvent::JobFailed {
                                job_id: job.id,
                                error: "Job cancelled by user".to_string(),
                            });
                        } else {
                            queue.set_active_status(TransferJobStatus::Failed);
                            let _ = event_tx.send(TransferEvent::JobFailed {
                                job_id: job.id,
                                error: err_msg,
                            });
                        }
                    }
                }

                // Pequeña pausa entre trabajos
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });

        self.active_worker_handle = Some(handle);
    }
}
