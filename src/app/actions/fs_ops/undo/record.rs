//! Files finished Transfer Engine jobs in the operation journal.

use crate::app::state::AppState;
use crate::fs::journal::{JobOrigin, from_transfer};
use crate::fs::transfer::job::{TransferJob, TransferResults};

/// Records what `job` did. `results` are the final ones of a completed job;
/// a failed or cancelled job is recorded with the files it finished before
/// stopping.
pub fn transfer_finished(state: &mut AppState, job: &TransferJob, results: &TransferResults) {
    let (direction, executed) = match state.journal.finish_job(job.id) {
        JobOrigin::Seen => return,
        JobOrigin::User => (None, None),
        JobOrigin::Journal(direction, command) => (Some(direction), Some(command)),
    };
    if let Some(command) = from_transfer(job, results, executed.as_ref()) {
        state.journal.settle(direction, command);
    }
}
