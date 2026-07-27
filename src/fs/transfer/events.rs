use super::job::{FailedFile, FileTransferResult, TransferResults};
use std::path::PathBuf;
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
    TransferStarted {
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
    /// Emitted at the end of a job when the
    /// [`super::policy::PromptPolicy`] has accumulated at
    /// least one `AccessDenied` failure. The UI is expected
    /// to show a single popup asking the user whether to
    /// retry those files as admin.
    ///
    /// `count` is pre-computed so the UI doesn't need to
    /// walk `files` just to display "N files failed".
    PermissionPrompt {
        job_id: Uuid,
        count: usize,
        files: Vec<PathBuf>,
    },
}
