//! Files finished Transfer Engine jobs in the operation journal.

use crate::app::state::AppState;
use crate::fs::journal::{JobOrigin, from_transfer};
use crate::fs::transfer::job::TransferResults;
use uuid::Uuid;

/// Records what job `job_id` did. `results` are the final ones of a
/// completed job; a failed or cancelled job is recorded with the files it
/// finished before stopping.
pub fn transfer_finished(state: &mut AppState, job_id: Uuid, results: Option<TransferResults>) {
    let Some(job) = state.transfer.as_ref().and_then(|ts| {
        ts.engine
            .queue
            .get_all()
            .into_iter()
            .find(|job| job.id == job_id)
    }) else {
        return;
    };
    let (direction, executed) = match state.journal.finish_job(job_id) {
        JobOrigin::Seen => return,
        JobOrigin::User => (None, None),
        JobOrigin::Journal(direction, command) => (Some(direction), Some(command)),
    };
    let results = results.unwrap_or_else(|| job.results.clone());
    if let Some(command) = from_transfer(&job, &results, executed.as_ref()) {
        state.journal.settle(direction, command);
    }
}
