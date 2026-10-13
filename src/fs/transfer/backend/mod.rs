//! Transfer backends (Strategy pattern).
//!
//! - [`local`] — filesystem worker (`TransferWorker`) for copy/move/delete
//! - [`remote`] — copy/move/delete on remote connections (SFTP, FTP, WebDAV, S3) emitting [`TransferEvent`]s
//! - [`ops_jobs`] — wipe / compress / extract (same Transfer UI, local)
//! - [`archive_vfs`] — copy out of / into archives, delete and move inside zip, tar and 7z files
//!
//! The engine picks a backend from job operation, optional remote endpoints
//! and whether a path lies inside an archive file.

mod archive_move;
#[cfg(test)]
mod archive_tests;
pub mod archive_vfs;
pub mod local;
pub mod ops_jobs;
pub mod remote;

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
        if job.remote.is_some() {
            return Err(anyhow::anyhow!(
                "{} is not available on remote connections; switch to a local panel",
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
        let conflicts = archive_vfs::ConflictSetup::of(&job);
        let control = JobControl::for_job(&job, event_tx.clone());
        let _ = event_tx.send(TransferEvent::JobStarted { job_id: job.id });
        let _ = event_tx.send(TransferEvent::ScanStarted { job_id: job.id });
        return match plan {
            Ok(plan) => archive_vfs::run(plan, control, conflicts).await,
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
    if let Some(remote) = job.remote.take() {
        let control = JobControl::for_job(&job, event_tx.clone());
        let _ = event_tx.send(TransferEvent::JobStarted { job_id: job.id });
        let _ = event_tx.send(TransferEvent::ScanStarted { job_id: job.id });
        remote::run_remote_job(remote::RemoteJob::of(job, remote), control).await
    } else {
        // Local copy/move/delete worker emits JobStarted / scan events itself.
        local::run_local_job(job, event_tx).await
    }
}
