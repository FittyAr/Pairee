//! Same-server remote fast rename/move via SFTP.

use super::super::super::job::TransferResults;
use super::super::super::worker::is_destination_parent_dir;
use crate::config::localization::t;
use crate::fs::ssh::SharedSshClient;
use crate::fs::transfer::control::JobControl;
use std::path::PathBuf;

pub fn fast_remote_rename(
    sources: Vec<PathBuf>,
    destination_dir: PathBuf,
    src_client: &SharedSshClient,
    dst_conn: &Option<SharedSshClient>,
    control: JobControl,
) -> Result<TransferResults, anyhow::Error> {
    let into_dir = is_destination_parent_dir(&sources, &destination_dir, |p| {
        crate::fs::ssh::is_dir_on(p, dst_conn)
    });
    control.run_each(&sources, |src| {
        let dst = if into_dir {
            destination_dir.join(crate::fs::file_name_lossy(src))
        } else {
            destination_dir.clone()
        };
        src_client
            .rename_move(src, &dst)
            .map(|()| dst)
            .map_err(|e| t("error_remote_move_failed").replacen("{}", &e.to_string(), 1))
    })
}
