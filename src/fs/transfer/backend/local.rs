//! Local filesystem transfer backend (delegates to [`TransferWorker`]).

use super::super::events::TransferEvent;
use super::super::job::{TransferJob, TransferResults};
use super::super::worker::TransferWorker;
use std::sync::Arc;
use tokio::sync::mpsc;

pub async fn run_local_job(
    job: TransferJob,
    event_tx: mpsc::UnboundedSender<TransferEvent>,
) -> Result<TransferResults, anyhow::Error> {
    debug_assert!(
        job.operation.uses_local_worker(),
        "ops backends must not enter the local transfer worker"
    );
    let worker = TransferWorker::new(
        job.id,
        job.operation,
        job.sources,
        job.destination,
        job.options,
        Arc::clone(&job.is_paused),
        Arc::clone(&job.is_cancelled),
        Arc::clone(&job.skip_file_flag),
        event_tx,
        job.active_conflict,
    );
    worker.run().await
}
