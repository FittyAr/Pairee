//! Secure wipe file operation runner.

use super::super::super::job::TransferResults;
use crate::config::localization::t;
use crate::fs::transfer::control::JobControl;
use std::path::PathBuf;

/// Wipes each source in turn (blocking); the first failure fails the job.
pub fn run_wipe(
    sources: Vec<PathBuf>,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    control.run_each(&sources, |path| {
        crate::fs::wipe::wipe_file(path)
            .map(|()| PathBuf::new())
            .map_err(|e| {
                t("error_wipe_failed_for")
                    .replacen("{}", &path.to_string_lossy(), 1)
                    .replacen("{}", &e.to_string(), 1)
            })
    })
}
