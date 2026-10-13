//! The verification step shared by both copy paths: progress events and the
//! comparison (a missing source hash is a failure, never a pass).

use super::super::control::JobControl;
use super::super::events::TransferEvent;
use super::super::options::TransferOptions;
use crate::config::localization::t;
use anyhow::anyhow;
use std::path::Path;

/// Announces the verification of the copy of `src`.
pub fn started(ctl: &JobControl, src: &Path, options: &TransferOptions) {
    ctl.emit(TransferEvent::VerifyStarted {
        job_id: ctl.job_id,
        file: src.to_path_buf(),
        algorithm: options.hash_algorithm.as_str().to_string(),
    });
}

/// Reports the verified bytes and compares the hashes; returns the
/// destination hash when they match.
pub fn compare(
    ctl: &JobControl,
    size: u64,
    src_hash: Option<&str>,
    dst_hash: String,
) -> anyhow::Result<Option<String>> {
    ctl.emit(TransferEvent::VerifyProgress {
        job_id: ctl.job_id,
        bytes_verified: size,
        bytes_total: size,
    });
    match src_hash {
        Some(expected) if expected == dst_hash => Ok(Some(dst_hash)),
        _ => Err(anyhow!(t("transfer_verify_mismatch"))),
    }
}
