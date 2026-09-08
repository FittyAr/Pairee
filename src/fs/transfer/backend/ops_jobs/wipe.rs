//! Secure wipe file operation runner.

use super::super::super::job::{FileTransferResult, TransferResults};
use super::super::BackendControl;
use super::common::{
    complete_ok, emit_file_completed, emit_file_started, emit_scan_complete, fail_file,
};
use crate::config::localization::t;
use anyhow::anyhow;
use std::path::PathBuf;
use std::time::Instant;

pub async fn run_wipe(
    sources: Vec<PathBuf>,
    control: BackendControl,
) -> Result<TransferResults, anyhow::Error> {
    let total = sources.len();
    emit_scan_complete(&control, total, 0);

    let mut results = TransferResults::default();

    for (idx, path) in sources.iter().enumerate() {
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }
        control.wait_if_paused();
        if control.cancelled() {
            return Err(anyhow!("Job cancelled"));
        }

        let start = Instant::now();
        emit_file_started(&control, path, idx);

        let wipe_path = path.clone();
        let wipe_res = tokio::task::spawn_blocking(move || crate::fs::wipe::wipe_file(&wipe_path))
            .await
            .map_err(|e| anyhow!("wipe task join error: {e}"))?;

        match wipe_res {
            Ok(()) => {
                let result = FileTransferResult {
                    src: path.clone(),
                    dst: PathBuf::new(),
                    size: 0,
                    src_hash: None,
                    dst_hash: None,
                    verified: true,
                    duration: start.elapsed(),
                };
                results.completed_files.push(result.clone());
                emit_file_completed(&control, result);
            }
            Err(e) => {
                let err_msg = t("error_wipe_failed_for")
                    .replacen("{}", &path.to_string_lossy(), 1)
                    .replacen("{}", &e.to_string(), 1);
                return fail_file(&control, &mut results, path.clone(), err_msg);
            }
        }
    }

    complete_ok(&control, results)
}
