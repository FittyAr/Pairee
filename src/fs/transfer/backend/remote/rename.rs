//! Same-server remote move: one server-side rename per source.

use super::super::super::job::TransferResults;
use crate::config::localization::t;
use crate::fs::remote::RemoteConn;
use crate::fs::transfer::control::JobControl;
use std::path::PathBuf;

/// Renames each source to its target (`roots`, see
/// [`crate::fs::transfer::scan::top_targets`]) on `client`.
pub fn fast_remote_rename(
    roots: Vec<(PathBuf, PathBuf)>,
    client: &RemoteConn,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    let (sources, targets): (Vec<PathBuf>, Vec<PathBuf>) = roots.into_iter().unzip();
    // `run_each` visits the sources in order.
    let mut targets = targets.into_iter();
    control.run_each(&sources, |src| {
        let dst = targets.next().unwrap_or_default();
        client
            .rename(src, &dst)
            .map(|()| dst)
            .map_err(|e| t("error_remote_move_failed").replacen("{}", &e.to_string(), 1))
    })
}
