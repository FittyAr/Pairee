//! SSH remote deletion operations.

use super::super::super::job::{SshEndpoints, TransferResults};
use crate::fs::transfer::control::JobControl;
use anyhow::anyhow;
use std::path::PathBuf;

pub fn run_ssh_delete(
    sources: Vec<PathBuf>,
    ssh: SshEndpoints,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    let client = ssh
        .src
        .or(ssh.dst)
        .ok_or_else(|| anyhow!("SSH delete requires a connection"))?;
    control.run_each(&sources, |path| {
        client
            .delete_recursive(path)
            .map(|()| PathBuf::new())
            .map_err(|e| e.to_string())
    })
}
