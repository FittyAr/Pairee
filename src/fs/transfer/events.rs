use super::job::{FailedFile, FileTransferResult, TransferResults};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum TransferEvent {
    JobStarted {
        job_id: Uuid,
    },
    ScanStarted {
        job_id: Uuid,
    },
    ScanProgress {
        job_id: Uuid,
        files_found: usize,
    },
    ScanComplete {
        job_id: Uuid,
        total_files: usize,
        total_bytes: u64,
    },
    FileStarted {
        job_id: Uuid,
        file: PathBuf,
        index: usize,
    },
    FileProgress {
        job_id: Uuid,
        bytes_copied: u64,
        bytes_total: u64,
    },
    FileCompleted {
        job_id: Uuid,
        result: FileTransferResult,
    },
    /// One captured stdout/stderr line from ApplyCommand (may include SGR).
    CommandOutput {
        job_id: Uuid,
        line: String,
    },
    FileFailed {
        job_id: Uuid,
        error: FailedFile,
    },
    FileSkipped {
        job_id: Uuid,
        file: PathBuf,
        reason: String,
    },
    VerifyStarted {
        job_id: Uuid,
        file: PathBuf,
        algorithm: String,
    },
    VerifyProgress {
        job_id: Uuid,
        bytes_verified: u64,
        bytes_total: u64,
    },
    JobCompleted {
        job_id: Uuid,
        results: TransferResults,
    },
    JobFailed {
        job_id: Uuid,
        error: String,
    },
    SpeedUpdate {
        job_id: Uuid,
        bytes_per_second: f64,
        eta_seconds: Option<u64>,
    },
    ConflictDetected {
        job_id: Uuid,
        file: PathBuf,
        conflict: super::conflict::ConflictInfo,
    },
}

impl TransferEvent {
    /// The job this event belongs to.
    pub fn job_id(&self) -> Uuid {
        match self {
            Self::JobStarted { job_id }
            | Self::ScanStarted { job_id }
            | Self::ScanProgress { job_id, .. }
            | Self::ScanComplete { job_id, .. }
            | Self::FileStarted { job_id, .. }
            | Self::FileProgress { job_id, .. }
            | Self::FileCompleted { job_id, .. }
            | Self::CommandOutput { job_id, .. }
            | Self::FileFailed { job_id, .. }
            | Self::FileSkipped { job_id, .. }
            | Self::VerifyStarted { job_id, .. }
            | Self::VerifyProgress { job_id, .. }
            | Self::JobCompleted { job_id, .. }
            | Self::JobFailed { job_id, .. }
            | Self::SpeedUpdate { job_id, .. }
            | Self::ConflictDetected { job_id, .. } => *job_id,
        }
    }
}

/// Minimum interval between forwarded progress events of one job.
pub const PROGRESS_MIN_INTERVAL: Duration = Duration::from_millis(50);

/// Sender for [`TransferEvent`]s that coalesces high-frequency progress
/// updates (one per 64 KiB chunk otherwise), so a fast copy cannot flood the
/// UI channel: at most one progress event per [`PROGRESS_MIN_INTERVAL`] is
/// forwarded, plus the final one of each file. All other events pass through.
#[derive(Debug, Clone)]
pub struct EventSender {
    tx: mpsc::UnboundedSender<TransferEvent>,
    epoch: Instant,
    /// Milliseconds since `epoch` of the last forwarded progress event (+1;
    /// 0 = none yet). Shared by clones.
    last_progress_ms: Arc<AtomicU64>,
}

impl EventSender {
    pub fn channel() -> (Self, mpsc::UnboundedReceiver<TransferEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let sender = Self {
            tx,
            epoch: Instant::now(),
            last_progress_ms: Arc::new(AtomicU64::new(0)),
        };
        (sender, rx)
    }

    /// Forwards `event` (unless coalesced). `false` when the UI side is gone.
    pub fn send(&self, event: TransferEvent) -> bool {
        self.should_drop(&event) || self.tx.send(event).is_ok()
    }

    fn should_drop(&self, event: &TransferEvent) -> bool {
        let finished = match event {
            TransferEvent::FileProgress {
                bytes_copied,
                bytes_total,
                ..
            } => bytes_copied >= bytes_total,
            TransferEvent::VerifyProgress {
                bytes_verified,
                bytes_total,
                ..
            } => bytes_verified >= bytes_total,
            TransferEvent::ScanProgress { .. } => false,
            _ => return false,
        };
        let now = self.epoch.elapsed().as_millis() as u64 + 1;
        let last = self.last_progress_ms.load(Ordering::Relaxed);
        if !finished
            && last != 0
            && now.saturating_sub(last) < PROGRESS_MIN_INTERVAL.as_millis() as u64
        {
            return true;
        }
        self.last_progress_ms.store(now, Ordering::Relaxed);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress(copied: u64, total: u64) -> TransferEvent {
        TransferEvent::FileProgress {
            job_id: Uuid::nil(),
            bytes_copied: copied,
            bytes_total: total,
        }
    }

    #[test]
    fn progress_bursts_are_coalesced_but_final_and_control_events_pass() {
        let (tx, mut rx) = EventSender::channel();
        for i in 0..1000 {
            tx.send(progress(i, 10_000));
        }
        tx.send(progress(10_000, 10_000));
        assert!(tx.send(TransferEvent::JobStarted {
            job_id: Uuid::nil(),
        }));
        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }
        assert!(events.len() < 10, "burst coalesced: {}", events.len());
        assert!(matches!(
            events[events.len() - 2],
            TransferEvent::FileProgress {
                bytes_copied: 10_000,
                ..
            }
        ));
        assert!(matches!(
            events.last(),
            Some(TransferEvent::JobStarted { .. })
        ));
    }
}
