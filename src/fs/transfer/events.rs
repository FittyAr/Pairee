use std::path::PathBuf;
use uuid::Uuid;
use super::job::{FileTransferResult, FailedFile, TransferResults};

#[derive(Debug, Clone)]
pub enum TransferEvent {
    JobStarted {
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
    },
    VerifyProgress {
        job_id: Uuid,
        files_verified: usize,
        total: usize,
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

#[derive(Debug, Clone)]
pub enum TransferCommand {
    Pause,
    Resume,
    Cancel,
    SkipFile,
    ResolveConflict {
        resolution: super::conflict::ConflictResolution,
    },
}
