//! [`JobControl`]: the run-time handles of one transfer job (id, pause /
//! cancel / skip flags and the event channel), passed as one parameter
//! object through the worker phases and the copy pipeline.

use super::events::{EventSender, TransferEvent};
use super::job::{FailedFile, FileTransferResult, SkippedFile, TransferJob, TransferResults};
use anyhow::anyhow;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use uuid::Uuid;

/// How often a paused job checks whether it may continue.
const PAUSE_POLL_ASYNC: Duration = Duration::from_millis(100);
const PAUSE_POLL_BLOCKING: Duration = Duration::from_millis(50);

/// Reason recorded for files skipped from the UI.
const SKIPPED_BY_USER: &str = "Skipped by user";

#[derive(Debug, Clone)]
pub struct JobControl {
    pub job_id: Uuid,
    pub is_paused: Arc<AtomicBool>,
    pub is_cancelled: Arc<AtomicBool>,
    pub skip_file_flag: Arc<AtomicBool>,
    pub event_tx: EventSender,
}

impl JobControl {
    /// Handles of `job`, reporting to `event_tx`.
    pub fn for_job(job: &TransferJob, event_tx: EventSender) -> Self {
        Self {
            job_id: job.id,
            is_paused: Arc::clone(&job.is_paused),
            is_cancelled: Arc::clone(&job.is_cancelled),
            skip_file_flag: Arc::clone(&job.skip_file_flag),
            event_tx,
        }
    }

    /// Sends `event` (a closed channel is ignored: the UI is gone).
    pub fn emit(&self, event: TransferEvent) {
        let _ = self.event_tx.send(event);
    }

    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::Relaxed)
    }

    /// `Err` once the job was cancelled.
    pub fn ensure_running(&self) -> anyhow::Result<()> {
        if self.is_cancelled() {
            Err(anyhow!("Job cancelled"))
        } else {
            Ok(())
        }
    }

    /// Waits (async) while paused; `Err` when cancelled.
    pub async fn wait_if_paused(&self) -> anyhow::Result<()> {
        self.ensure_running()?;
        while self.is_paused.load(Ordering::Relaxed) {
            self.ensure_running()?;
            tokio::time::sleep(PAUSE_POLL_ASYNC).await;
        }
        Ok(())
    }

    /// Blocking-thread version of [`Self::wait_if_paused`] (copy pipeline).
    pub fn wait_if_paused_blocking(&self) -> anyhow::Result<()> {
        let cancelled = || anyhow!("Transfer cancelled");
        if self.is_cancelled() {
            return Err(cancelled());
        }
        while self.is_paused.load(Ordering::Relaxed) {
            if self.is_cancelled() {
                return Err(cancelled());
            }
            std::thread::sleep(PAUSE_POLL_BLOCKING);
        }
        Ok(())
    }

    /// When the user asked to skip the current file, records `src` as
    /// skipped and returns `true`.
    pub fn take_user_skip(&self, src: &Path, results: &mut TransferResults) -> bool {
        if !self.skip_file_flag.swap(false, Ordering::Relaxed) {
            return false;
        }
        results.skipped_files.push(SkippedFile {
            src: src.to_path_buf(),
            reason: SKIPPED_BY_USER.to_string(),
        });
        self.emit(TransferEvent::FileSkipped {
            job_id: self.job_id,
            file: src.to_path_buf(),
            reason: SKIPPED_BY_USER.to_string(),
        });
        true
    }

    pub fn file_started(&self, file: &Path, index: usize) {
        self.emit(TransferEvent::FileStarted {
            job_id: self.job_id,
            file: file.to_path_buf(),
            index,
        });
    }

    /// Records and reports a failed file.
    pub fn file_failed(&self, results: &mut TransferResults, failed: FailedFile) {
        results.failed_files.push(failed.clone());
        self.emit(TransferEvent::FileFailed {
            job_id: self.job_id,
            error: failed,
        });
    }

    /// Records and reports a completed file.
    pub fn file_completed(&self, results: &mut TransferResults, done: FileTransferResult) {
        results.completed_files.push(done.clone());
        self.emit(TransferEvent::FileCompleted {
            job_id: self.job_id,
            result: done,
        });
    }

    /// Reports the end of the job and returns its results.
    pub fn job_completed(&self, results: TransferResults) -> anyhow::Result<TransferResults> {
        self.emit(TransferEvent::JobCompleted {
            job_id: self.job_id,
            results: results.clone(),
        });
        Ok(results)
    }
}
