use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TransferJob {
    pub id: Uuid,
    pub operation: TransferOperation,
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub options: super::options::TransferOptions,
    pub status: TransferJobStatus,
    pub results: TransferResults,
    pub active_conflict: Arc<std::sync::Mutex<Option<super::conflict::ConflictResolution>>>,
}

impl TransferJob {
    pub fn new(
        operation: TransferOperation,
        sources: Vec<PathBuf>,
        destination: PathBuf,
        options: super::options::TransferOptions,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            operation,
            sources,
            destination,
            options,
            status: TransferJobStatus::Queued,
            results: TransferResults::default(),
            active_conflict: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            TransferJobStatus::Scanning
                | TransferJobStatus::Transferring
                | TransferJobStatus::Verifying
                | TransferJobStatus::Paused
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TransferJobStatus::Completed
                | TransferJobStatus::Failed
                | TransferJobStatus::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TransferOperation {
    Copy,
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TransferJobStatus {
    Queued,
    Scanning,
    Transferring,
    Verifying,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for TransferJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TransferJobStatus::Queued => "Queued",
            TransferJobStatus::Scanning => "Scanning",
            TransferJobStatus::Transferring => "Transferring",
            TransferJobStatus::Verifying => "Verifying",
            TransferJobStatus::Paused => "Paused",
            TransferJobStatus::Completed => "Completed",
            TransferJobStatus::Failed => "Failed",
            TransferJobStatus::Cancelled => "Cancelled",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TransferProgress {
    pub current_file: String,
    pub files_scanned: usize,
    pub files_total: usize,
    pub files_completed: usize,
    pub files_failed: usize,
    pub files_skipped: usize,
    pub bytes_total: u64,
    pub bytes_transferred: u64,
    pub bytes_per_second: f64,
    pub eta_seconds: Option<u64>,
}

impl TransferProgress {
    pub fn percent_bytes(&self) -> f32 {
        if self.bytes_total == 0 {
            0.0
        } else {
            (self.bytes_transferred as f32 / self.bytes_total as f32) * 100.0
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TransferResults {
    pub completed_files: Vec<FileTransferResult>,
    pub failed_files: Vec<FailedFile>,
    pub skipped_files: Vec<SkippedFile>,
}

#[derive(Debug, Clone)]
pub struct FileTransferResult {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub size: u64,
    pub src_hash: Option<String>,
    pub dst_hash: Option<String>,
    pub verified: bool,
    pub duration: std::time::Duration,
}

#[derive(Debug, Clone)]
pub struct FailedFile {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub error: String,
    pub retries: u32,
}

#[derive(Debug, Clone)]
pub struct SkippedFile {
    pub src: PathBuf,
    pub reason: String,
}
