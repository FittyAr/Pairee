//! Restore-from-trash runner (undo of "send to trash").

use super::super::super::job::TransferResults;
use crate::fs::journal::TrashIndex;
use crate::fs::transfer::control::JobControl;
use std::path::PathBuf;

/// Puts each trashed source back at its original path (blocking); the
/// first failure fails the job. The trash is listed once per job.
pub fn run_restore(
    sources: Vec<PathBuf>,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    let mut index: Option<TrashIndex> = None;
    control.run_each(&sources, |path| {
        let trash = match index.as_mut() {
            Some(trash) => trash,
            None => index.insert(TrashIndex::load()?),
        };
        trash.restore(path).map(|()| path.to_path_buf())
    })
}
