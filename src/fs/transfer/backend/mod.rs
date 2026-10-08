//! Transfer backends (Strategy pattern).
//!
//! - [`local`] — filesystem worker (`TransferWorker`) for copy/move/delete
//! - [`ssh`] — SFTP copy/move/delete emitting [`TransferEvent`]s
//! - [`ops_jobs`] — wipe / compress / extract (same Transfer UI, local)
//! - [`archive_vfs`] — copy out of / into archives, delete inside zip files
//!
//! The engine picks a backend from job operation, optional SSH endpoints
//! and whether a path lies inside an archive file.

#[cfg(test)]
mod archive_tests;
pub mod archive_vfs;
pub mod local;
pub mod ops_jobs;
pub mod ssh;

use super::events::TransferEvent;
use super::job::{TransferJob, TransferResults};
use crate::fs::transfer::control::JobControl;

/// Run the appropriate backend for a job (Strategy dispatch).
pub async fn run_job(
    job: TransferJob,
    event_tx: crate::fs::transfer::events::EventSender,
) -> Result<TransferResults, anyhow::Error> {
    // Wipe / compress / extract / apply-command are local-only Strategy backends.
    if job.operation.uses_ops_backend() {
        if job.ssh.is_some() {
            return Err(anyhow::anyhow!(
                "{} is not available over SSH; switch to a local panel",
                job.operation.label()
            ));
        }
        let control = JobControl::for_job(&job, event_tx.clone());
        let _ = event_tx.send(TransferEvent::JobStarted { job_id: job.id });
        let _ = event_tx.send(TransferEvent::ScanStarted { job_id: job.id });
        return ops_jobs::run_ops_job(
            job.operation,
            job.sources,
            job.destination,
            job.shell_template,
            control,
        )
        .await;
    }

    if let Some(plan) = archive_vfs::plan(&job) {
        let control = JobControl::for_job(&job, event_tx.clone());
        let _ = event_tx.send(TransferEvent::JobStarted { job_id: job.id });
        let _ = event_tx.send(TransferEvent::ScanStarted { job_id: job.id });
        return match plan {
            Ok(plan) => archive_vfs::run(plan, control).await,
            Err(reason) => {
                let _ = event_tx.send(TransferEvent::JobFailed {
                    job_id: job.id,
                    error: reason.clone(),
                });
                Err(anyhow::anyhow!(reason))
            }
        };
    }

    let mut job = job;
    if let Some(ssh) = job.ssh.take() {
        let control = JobControl::for_job(&job, event_tx.clone());
        let _ = event_tx.send(TransferEvent::JobStarted { job_id: job.id });
        let _ = event_tx.send(TransferEvent::ScanStarted { job_id: job.id });
        ssh::run_ssh_job(job.operation, job.sources, job.destination, ssh, control).await
    } else {
        // Local copy/move/delete worker emits JobStarted / scan events itself.
        local::run_local_job(job, event_tx).await
    }
}
