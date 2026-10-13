//! Deletion on a remote connection.

use super::super::super::job::{RemoteEndpoints, TransferResults};
use crate::fs::transfer::control::JobControl;
use anyhow::anyhow;
use std::path::PathBuf;

pub fn run_remote_delete(
    sources: Vec<PathBuf>,
    remote: RemoteEndpoints,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    let client = remote
        .src
        .or(remote.dst)
        .ok_or_else(|| anyhow!("Remote delete requires a connection"))?;
    control.run_each(&sources, |path| {
        client
            .remove_all(path)
            .map(|()| PathBuf::new())
            .map_err(|e| e.to_string())
    })
}
