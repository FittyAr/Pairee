//! Shared helpers and event broadcasters for ops jobs.

use super::super::super::events::TransferEvent;
use super::super::super::job::{FailedFile, FileTransferResult, TransferResults};
use super::super::BackendControl;
use anyhow::anyhow;
use std::path::{Path, PathBuf};

pub fn emit_scan_complete(control: &BackendControl, total_files: usize, total_bytes: u64) {
    let _ = control.event_tx.send(TransferEvent::ScanComplete {
        job_id: control.job_id,
        total_files,
        total_bytes,
    });
}

pub fn emit_file_started(control: &BackendControl, file: &Path, index: usize) {
    let _ = control.event_tx.send(TransferEvent::FileStarted {
        job_id: control.job_id,
        file: file.to_path_buf(),
        index,
    });
}

pub fn emit_file_completed(control: &BackendControl, result: FileTransferResult) {
    let _ = control.event_tx.send(TransferEvent::FileCompleted {
        job_id: control.job_id,
        result,
    });
}

pub fn fail_file(
    control: &BackendControl,
    results: &mut TransferResults,
    path: PathBuf,
    err_msg: String,
) -> Result<TransferResults, anyhow::Error> {
    let failed = FailedFile {
        src: path,
        dst: PathBuf::new(),
        error: err_msg.clone(),
        retries: 0,
    };
    results.failed_files.push(failed.clone());
    let _ = control.event_tx.send(TransferEvent::FileFailed {
        job_id: control.job_id,
        error: failed,
    });
    let _ = control.event_tx.send(TransferEvent::JobFailed {
        job_id: control.job_id,
        error: err_msg.clone(),
    });
    Err(anyhow!(err_msg))
}

pub fn complete_ok(
    control: &BackendControl,
    results: TransferResults,
) -> Result<TransferResults, anyhow::Error> {
    let _ = control.event_tx.send(TransferEvent::JobCompleted {
        job_id: control.job_id,
        results: results.clone(),
    });
    Ok(results)
}
