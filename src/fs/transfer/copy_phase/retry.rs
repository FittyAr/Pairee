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

/// Whether trying again cannot help: the file is not allowed, not there, a
/// folder is in the way, the volume is full or read-only, the name is
/// unusable. Network drops, timeouts and failed verifications are worth
/// another attempt.
fn is_permanent(error: &anyhow::Error) -> bool {
    use std::io::ErrorKind as K;
    error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<std::io::Error>())
        .any(|io| {
            matches!(
                io.kind(),
                K::PermissionDenied
                    | K::NotFound
                    | K::AlreadyExists
                    | K::InvalidInput
                    | K::InvalidFilename
                    | K::Unsupported
                    | K::IsADirectory
                    | K::NotADirectory
                    | K::DirectoryNotEmpty
                    | K::ReadOnlyFilesystem
                    | K::StorageFull
                    | K::QuotaExceeded
                    | K::FileTooLarge
            )
        })
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
                if is_permanent(&e) {
                    break;
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::transfer::job::{TransferJob, TransferOperation};
    use std::io;

    async fn attempts_before_giving_up(error: fn() -> anyhow::Error) -> (u32, TransferOutcome) {
        let (tx, mut rx) = crate::fs::transfer::events::EventSender::channel();
        tokio::spawn(async move { while rx.recv().await.is_some() {} });
        let options = TransferOptions {
            max_retries: 2,
            ..TransferOptions::default()
        };
        let job = TransferJob::new(
            TransferOperation::Copy,
            vec![],
            std::path::PathBuf::new(),
            options.clone(),
        );
        let ctl = JobControl::for_job(&job, tx);
        let copied = Arc::new(AtomicU64::new(0));
        let tries = std::cell::Cell::new(0);
        let outcome = with_retries(
            &options,
            &ctl,
            &copied,
            || {
                tries.set(tries.get() + 1);
                std::future::ready(Err::<Committed, _>(error()))
            },
            || {},
        )
        .await
        .unwrap();
        (tries.get(), outcome)
    }

    /// A permission error or a folder in the way fails at once; a dropped
    /// connection uses every retry.
    #[tokio::test]
    async fn permanent_errors_are_not_retried() {
        let denied = || anyhow::Error::new(io::Error::from(io::ErrorKind::PermissionDenied));
        let (tries, outcome) = attempts_before_giving_up(denied).await;
        assert_eq!(tries, 1);
        assert!(!outcome.success);

        let wrapped = || {
            anyhow::Error::new(io::Error::from(io::ErrorKind::AlreadyExists))
                .context("folder in the way")
        };
        assert_eq!(attempts_before_giving_up(wrapped).await.0, 1);

        let dropped = || anyhow::Error::new(io::Error::from(io::ErrorKind::ConnectionReset));
        assert_eq!(attempts_before_giving_up(dropped).await.0, 3);
        let other = || anyhow::anyhow!("hash mismatch");
        assert_eq!(attempts_before_giving_up(other).await.0, 3);
    }
}
