//! Transfer backends (Strategy pattern).
//!
//! - [`local`] — filesystem worker (`TransferWorker`) for copy/move/delete
//! - [`ssh`] — SFTP copy/move/delete emitting [`TransferEvent`]s
//! - [`ops_jobs`] — wipe / compress / extract (same Transfer UI, local)
//!
//! The engine picks a backend from job operation + optional SSH endpoints.

pub mod local;
pub mod ops_jobs;
pub mod ssh;

use super::events::TransferEvent;
use super::job::{TransferJob, TransferResults};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use uuid::Uuid;

/// Shared control surface for any backend run.
pub struct BackendControl {
    pub job_id: Uuid,
    pub is_paused: Arc<AtomicBool>,
    pub is_cancelled: Arc<AtomicBool>,
    pub event_tx: mpsc::UnboundedSender<TransferEvent>,
}

impl BackendControl {
    pub fn cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::Relaxed)
    }

    pub fn wait_if_paused(&self) {
        while self.is_paused.load(Ordering::Relaxed) {
            if self.cancelled() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}

/// Run the appropriate backend for a job (Strategy dispatch).
pub async fn run_job(
    job: TransferJob,
    event_tx: mpsc::UnboundedSender<TransferEvent>,
) -> Result<TransferResults, anyhow::Error> {
    // Wipe / compress / extract are local-only Strategy backends (same UI).
    if job.operation.uses_ops_backend() {
        if job.ssh.is_some() {
            return Err(anyhow::anyhow!(
                "{} is not available over SSH; switch to a local panel",
                job.operation.label()
            ));
        }
        let control = BackendControl {
            job_id: job.id,
            is_paused: Arc::clone(&job.is_paused),
            is_cancelled: Arc::clone(&job.is_cancelled),
            event_tx: event_tx.clone(),
        };
        let _ = event_tx.send(TransferEvent::JobStarted { job_id: job.id });
        let _ = event_tx.send(TransferEvent::ScanStarted { job_id: job.id });
        return ops_jobs::run_ops_job(job.operation, job.sources, job.destination, control).await;
    }

    if let Some(ssh) = job.ssh {
        let control = BackendControl {
            job_id: job.id,
            is_paused: Arc::clone(&job.is_paused),
            is_cancelled: Arc::clone(&job.is_cancelled),
            event_tx: event_tx.clone(),
        };
        let _ = event_tx.send(TransferEvent::JobStarted { job_id: job.id });
        let _ = event_tx.send(TransferEvent::ScanStarted { job_id: job.id });
        ssh::run_ssh_job(job.operation, job.sources, job.destination, ssh, control).await
    } else {
        // Local copy/move/delete worker emits JobStarted / scan events itself.
        local::run_local_job(job, event_tx).await
    }
}
