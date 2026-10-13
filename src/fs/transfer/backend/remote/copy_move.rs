//! Remote copy and move: the shared scan and copy phase over the remote and
//! local [`Vfs`] adapters, so remote transfers get what local ones have —
//! conflict resolution, retries, verification, times and permissions,
//! atomic writes (`.pairee-part` + rename) and moves that only delete a
//! source once its copy is complete. Links to folders in a remote source
//! are skipped with a warning (or walked when following links) instead of
//! aborting the job.

use super::super::super::job::{RemoteEndpoints, TransferOperation, TransferResults};
use super::fast_remote_rename;
use crate::fs::remote::endpoint_vfs;
use crate::fs::transfer::control::JobControl;
use crate::fs::transfer::copy_phase::{CopyRequest, Endpoints, run_copy_phase};
use crate::fs::transfer::scan::{ScanOutcome, ScanRequest, scan, top_targets};
use anyhow::anyhow;

use super::RemoteJob;

pub async fn run_remote_copy_move(
    job: RemoteJob,
    control: JobControl,
) -> anyhow::Result<TransferResults> {
    let RemoteEndpoints { src, dst } = job.remote.clone();
    if src.is_none() && dst.is_none() {
        return Err(anyhow!("Remote transfer requires at least one connection"));
    }
    let same_server = matches!((&src, &dst), (Some(a), Some(b)) if a.is_same_server(b));
    if job.operation == TransferOperation::Move && same_server {
        let src_client = src.clone().unwrap_or_else(|| unreachable!("checked above"));
        return blocking(move || {
            fast_remote_rename(job.sources, job.destination, &src_client, &dst, control)
        })
        .await;
    }
    let endpoints = Endpoints {
        src: endpoint_vfs(&src),
        dst: endpoint_vfs(&dst),
        same_fs: same_server,
        local: false,
    };
    let scan = scan_blocking(&job, &endpoints, &control).await?;
    let request = CopyRequest {
        operation: job.operation,
        scan,
        options: &job.options,
        endpoints,
        slot: job.slot.clone(),
    };
    let results = run_copy_phase(request, &control, TransferResults::default()).await?;
    control.job_completed(results)
}

/// The scan reads the server: it runs on a blocking thread.
async fn scan_blocking(
    job: &RemoteJob,
    endpoints: &Endpoints,
    control: &JobControl,
) -> anyhow::Result<ScanOutcome> {
    let (job, fs, ctl) = (job.clone(), endpoints.clone(), control.clone());
    blocking(move || {
        let roots = top_targets(
            &job.sources,
            &job.destination,
            job.operation,
            fs.dst.as_ref(),
        );
        let request = ScanRequest {
            roots: &roots,
            operation: job.operation,
            options: &job.options,
            src_fs: fs.src.as_ref(),
            dst_fs: fs.dst.as_ref(),
            same_fs: fs.same_fs,
            dst_links: false,
        };
        scan(&request, &ctl)
    })
    .await
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> anyhow::Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| anyhow!("Remote transfer task failed: {e}"))?
}
