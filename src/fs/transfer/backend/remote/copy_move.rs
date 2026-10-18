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
use std::path::PathBuf;

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
    let endpoints = Endpoints {
        src: endpoint_vfs(&src),
        dst: endpoint_vfs(&dst),
        same_fs: same_server,
        local: false,
    };
    // A move within one server is a rename, unless something is already
    // at a target: many servers rename over it, so the conflict setting
    // goes through the copy phase instead.
    if job.operation == TransferOperation::Move
        && same_server
        && let Some(roots) = free_targets(&job, &endpoints).await?
    {
        let src_client = src.clone().unwrap_or_else(|| unreachable!("checked above"));
        return blocking(move || fast_remote_rename(roots, &src_client, control)).await;
    }
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

/// Each source of `job` with its target, when nothing is at any target.
async fn free_targets(
    job: &RemoteJob,
    endpoints: &Endpoints,
) -> anyhow::Result<Option<Vec<(PathBuf, PathBuf)>>> {
    let (job, fs) = (job.clone(), endpoints.clone());
    blocking(move || {
        let roots = top_targets(
            &job.sources,
            &job.destination,
            job.operation,
            job.destination_kind,
            fs.dst.as_ref(),
        );
        let free = roots.iter().all(|(_, dst)| !fs.dst.exists(dst));
        Ok(free.then_some(roots))
    })
    .await
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
            job.destination_kind,
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
        let out = scan(&request, &ctl)?;
        out.announce(&ctl, 0);
        Ok(out)
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
