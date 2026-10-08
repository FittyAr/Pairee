//! Local filesystem transfer backend (delegates to [`TransferWorker`]).

use super::super::job::{TransferJob, TransferResults};
use super::super::worker::TransferWorker;

pub async fn run_local_job(
    job: TransferJob,
    event_tx: crate::fs::transfer::events::EventSender,
) -> Result<TransferResults, anyhow::Error> {
    debug_assert!(
        job.operation.uses_local_worker(),
        "ops backends must not enter the local transfer worker"
    );
    let worker = TransferWorker::for_job(job, event_tx);
    worker.run().await
}
