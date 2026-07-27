use std::sync::Arc;

use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::events::TransferEvent;
use super::job::{TransferJob, TransferJobStatus};
use super::policy::{PromptPolicy, TransferPolicy};
use super::queue::TransferQueue;
use super::worker::TransferWorker;

pub struct TransferEngine {
    pub queue: TransferQueue,
    event_tx: mpsc::UnboundedSender<TransferEvent>,
    active_coordinator_handle: Option<JoinHandle<()>>,
    /// Strategy the engine uses to react to file-level
    /// failures. Defaults to a [`PromptPolicy`] so the
    /// retry-as-admin prompt is wired end-to-end from the
    /// start. Pass [`super::policy::LoggingPolicy`]
    /// (or any custom impl) via [`Self::with_policy`] for
    /// headless contexts.
    policy: Arc<dyn TransferPolicy>,
}

impl TransferEngine {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<TransferEvent>) {
        Self::with_policy(None)
    }

    /// Build an engine with a custom policy. Pass `None`
    /// to use the default [`PromptPolicy`].
    pub fn with_policy(
        policy: Option<Arc<dyn TransferPolicy>>,
    ) -> (Self, mpsc::UnboundedReceiver<TransferEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let engine = Self {
            queue: TransferQueue::new(),
            event_tx,
            active_coordinator_handle: None,
            policy: policy.unwrap_or_else(|| Arc::new(PromptPolicy::new())),
        };
        (engine, event_rx)
    }

    pub fn submit_job(&mut self, job: TransferJob) {
        // Each new job starts with a clean policy slate.
        self.policy.reset();
        self.queue.enqueue(job);
        self.trigger_processing_loop();
    }

    pub fn trigger_processing_loop(&mut self) {
        if self.active_coordinator_handle.is_some() {
            return;
        }

        let queue = self.queue.clone();
        let event_tx = self.event_tx.clone();
        // Clone the policy for the coordinator task so the
        // worker spawned inside the loop can pass it down
        // to its `TransferWorker`.
        let policy = Arc::clone(&self.policy);

        let handle = tokio::spawn(async move {
            let mut active_worker: Option<(uuid::Uuid, JoinHandle<()>)> = None;

            loop {
                // Verificar si el trabajador activo ha terminado (o entrado en pánico/abortado)
                if let Some((job_id, ref worker_handle)) = active_worker {
                    if worker_handle.is_finished() {
                        let jobs = queue.get_all();
                        if let Some(job) = jobs.iter().find(|j| j.id == job_id) {
                            if !job.is_terminal() {
                                // Terminó de forma inesperada (p. ej. pánico)
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
                        // Fresh Arc clone per job so the
                        // worker task can take ownership
                        // without disturbing the loop's
                        // own reference to the policy.
                        let policy_for_worker = Arc::clone(&policy);

                        let worker_handle = tokio::spawn(async move {
                            let worker = TransferWorker::new(
                                job.id,
                                job.operation,
                                job.sources,
                                job.destination,
                                job.src_endpoint,
                                job.dst_endpoint,
                                job.options.clone(),
                                Arc::clone(&job.is_paused),
                                Arc::clone(&job.is_cancelled),
                                Arc::clone(&job.skip_file_flag),
                                event_tx_clone.clone(),
                                job.active_conflict.clone(),
                                policy_for_worker,
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
                                    let is_cancel = err_msg.contains("cancelled");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::transfer::policy::{FileError, RetryRequest};
    use std::path::{Path, PathBuf};

    #[test]
    fn default_engine_can_be_built() {
        // Smoke test: the default `new()` builds an
        // engine without panicking. The default policy
        // is a `PromptPolicy` whose `finalize()` returns
        // the empty list; the actual prompt emission is
        // driven by the worker at the end of a job and
        // is covered by integration tests.
        let (_engine, _rx) = TransferEngine::new();
    }

    #[test]
    fn with_policy_accepts_a_custom_trait_object() {
        // We don't keep the `Arc` around: we just want
        // to confirm the constructor accepts one.
        let _engine =
            TransferEngine::with_policy(Some(Arc::new(crate::fs::transfer::policy::PromptPolicy::new())));
    }

    #[test]
    fn policy_file_error_access_denied_helper_is_observable() {
        // The helper exists so callers can filter
        // `AccessDenied` without matching the whole enum
        // by hand.
        let denied = FileError::AccessDenied;
        let other = FileError::IoError("nope".to_string());
        assert!(denied.is_access_denied());
        assert!(!other.is_access_denied());
    }

    #[test]
    fn retry_request_carries_original_path_and_error() {
        let r = RetryRequest {
            original_path: PathBuf::from("/some/file"),
            error: "Access denied".to_string(),
        };
        assert_eq!(r.original_path, PathBuf::from("/some/file"));
        assert_eq!(r.error, "Access denied");
    }

    #[test]
    fn file_error_categorises_known_messages() {
        // Public API smoke test of `FileError::from_error_message`.
        assert_eq!(
            FileError::from_error_message("Access is denied"),
            FileError::AccessDenied
        );
        assert_eq!(
            FileError::from_error_message("file not found"),
            FileError::NotFound
        );
        match FileError::from_error_message("random io error") {
            FileError::IoError(s) => assert_eq!(s, "random io error"),
            other => panic!("expected IoError, got {:?}", other),
        }
    }

    #[test]
    fn prompt_policy_collects_access_denied_only() {
        use crate::fs::transfer::policy::{PromptPolicy, TransferPolicy};
        let policy = PromptPolicy::new();
        // Two AccessDenied + one NotFound + one IoError.
        policy.on_file_error(Path::new("/a"), &FileError::AccessDenied);
        policy.on_file_error(Path::new("/b"), &FileError::AccessDenied);
        policy.on_file_error(Path::new("/c"), &FileError::NotFound);
        policy.on_file_error(
            Path::new("/d"),
            &FileError::IoError("nope".to_string()),
        );
        // `finalize` drains the accumulated list. Only
        // the two AccessDenied entries should be there.
        let snap = policy.finalize();
        assert_eq!(snap.len(), 2);
        let paths: std::collections::HashSet<_> = snap
            .iter()
            .map(|r| r.original_path.clone())
            .collect();
        assert!(paths.contains(&PathBuf::from("/a")));
        assert!(paths.contains(&PathBuf::from("/b")));
        // Reset clears state.
        policy.reset();
        assert!(policy.finalize().is_empty());
    }
}
