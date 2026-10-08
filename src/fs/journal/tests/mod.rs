//! Journal core tests: stacks, inverses, preconditions, and undo/redo runs
//! on real temporary folders through the Transfer Engine backends and the
//! multi-rename executor.

mod history;
mod renames;
mod transfers;

use super::{Direction, FsCommand, from_transfer};
use crate::fs::transfer::job::{TransferJob, TransferResults};
use std::path::{Path, PathBuf};

/// Runs `job` on its Transfer Engine backend and returns its results.
async fn run_job(mut job: TransferJob) -> TransferResults {
    let (tx, mut rx) = crate::fs::transfer::events::EventSender::channel();
    tokio::spawn(async move { while rx.recv().await.is_some() {} });
    let results = crate::fs::transfer::backend::run_job(job.clone(), tx)
        .await
        .expect("job runs");
    job.results = results.clone();
    results
}

/// Runs `job` and records it as `from_transfer` would after it finished.
async fn run_recorded(job: TransferJob, executed: Option<&FsCommand>) -> Option<FsCommand> {
    let results = run_job(job.clone()).await;
    from_transfer(&job, &results, executed)
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, text).unwrap();
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

fn p(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

/// The runnable inverse of `entry` (panics when nothing can run).
fn runnable_inverse(entry: &FsCommand) -> FsCommand {
    super::check(&entry.inverse().expect("undoable"))
        .runnable
        .expect("something to run")
}

const UNDO: Direction = Direction::Undo;
