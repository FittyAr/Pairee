//! Retries of one file transfer, shared by the local and the stream copy:
//! each attempt starts the byte count of the file again (so the job's
//! progress never passes 100 %), failures wait with exponential backoff,
//! and a cancelled or abandoned transfer cleans up its temporary file.

use super::super::control::JobControl;
use super::super::options::TransferOptions;
use super::super::part::Replaced;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Longest wait between two attempts.
const MAX_BACKOFF_MS: u64 = 30_000;

/// How one file transfer ended.
#[derive(Debug, Default)]
pub struct TransferOutcome {
    pub success: bool,
    pub last_error: String,
    pub retries: u32,
    pub src_hash: Option<String>,
    pub dst_hash: Option<String>,
    /// The copy was read back and matched the source.
    pub verified: bool,
    pub replaced: Replaced,
}

/// A transfer committed at its destination.
pub struct Committed {
    pub src_hash: Option<String>,
    pub dst_hash: Option<String>,
    pub replaced: Replaced,
}

/// Runs `attempt` until it succeeds or the job's retries are used up;
/// `abandon` cleans up after the last failure or a cancellation. `Err` only
/// when the job is cancelled.
pub async fn with_retries<Fut>(
    options: &TransferOptions,
    ctl: &JobControl,
    copied: &Arc<AtomicU64>,
    mut attempt: impl FnMut() -> Fut,
    abandon: impl Fn(),
) -> anyhow::Result<TransferOutcome>
where
    Fut: Future<Output = anyhow::Result<Committed>>,
{
    let mut outcome = TransferOutcome::default();
    // Bytes of the job before this file: a retry starts counting again.
    let before = copied.load(Ordering::SeqCst);
    while outcome.retries <= options.max_retries {
        if let Err(e) = ctl.ensure_running() {
            abandon();
            return Err(e);
        }
        copied.store(before, Ordering::SeqCst);
        match attempt().await {
            Ok(done) => {
                outcome.success = true;
                outcome.verified = done.dst_hash.is_some();
                outcome.src_hash = done.src_hash;
                outcome.dst_hash = done.dst_hash;
                outcome.replaced = done.replaced;
                return Ok(outcome);
            }
            Err(e) if ctl.is_cancelled() => {
                abandon();
                return Err(e);
            }
            Err(e) => {
                outcome.retries += 1;
                outcome.last_error = e.to_string();
                if outcome.retries <= options.max_retries {
                    let backoff_ms = 100u64.saturating_mul(1u64 << outcome.retries.min(20));
                    tokio::time::sleep(Duration::from_millis(backoff_ms.min(MAX_BACKOFF_MS))).await;
                }
            }
        }
    }
    abandon();
    Ok(outcome)
}
