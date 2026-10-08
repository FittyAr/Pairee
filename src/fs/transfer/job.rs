use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// Optional SSH endpoints for a transfer (Strategy: non-local backend).
#[derive(Debug, Clone)]
pub struct SshEndpoints {
    pub src: Option<crate::fs::ssh::SharedSshClient>,
    pub dst: Option<crate::fs::ssh::SharedSshClient>,
}

#[derive(Debug, Clone)]
pub struct TransferJob {
    pub id: Uuid,
    pub operation: TransferOperation,
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub options: super::options::TransferOptions,
    pub status: TransferJobStatus,
    pub results: TransferResults,
    pub progress: Option<TransferProgress>,
    pub log_lines: Vec<String>,
    pub is_paused: Arc<std::sync::atomic::AtomicBool>,
    pub is_cancelled: Arc<std::sync::atomic::AtomicBool>,
    pub skip_file_flag: Arc<std::sync::atomic::AtomicBool>,
    pub active_conflict: Arc<crate::fs::transfer::conflict_slot::ConflictSlot>,
    /// When `Some`, the engine runs the SSH backend instead of the local worker.
    pub ssh: Option<SshEndpoints>,
    /// Shell command template for [`TransferOperation::ApplyCommand`] (`%f` = path).
    pub shell_template: Option<String>,
    /// Copy/Move of explicit `(source file, destination file)` pairs instead
    /// of `sources` into `destination` (used to undo/redo operations).
    pub pairs: Option<Vec<(PathBuf, PathBuf)>>,
    /// Folders removed (when empty, deepest first) after the job.
    pub prune_dirs: Vec<PathBuf>,
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
            progress: None,
            log_lines: Vec::new(),
            is_paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            is_cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            skip_file_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            active_conflict: Arc::default(),
            ssh: None,
            shell_template: None,
            pairs: None,
            prune_dirs: Vec::new(),
        }
    }

    /// Copy/Move job of explicit file pairs (local only); every destination
    /// folder that is missing is created.
    pub fn for_pairs(
        operation: TransferOperation,
        pairs: Vec<(PathBuf, PathBuf)>,
        options: super::options::TransferOptions,
    ) -> Self {
        let sources = pairs.iter().map(|(from, _)| from.clone()).collect();
        let mut job = Self::new(operation, sources, PathBuf::new(), options);
        job.pairs = Some(pairs);
        job
    }

    /// Folders to remove after the job when they are empty.
    pub fn with_prune_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.prune_dirs = dirs;
        self
    }

    /// Attach SSH endpoints (copy/move/delete over SFTP).
    pub fn with_ssh(mut self, ssh: SshEndpoints) -> Self {
        self.ssh = Some(ssh);
        self
    }

    /// Attach shell template for ApplyCommand (`%f` expands to each source path).
    pub fn with_shell_template(mut self, template: impl Into<String>) -> Self {
        self.shell_template = Some(template.into());
        self
    }

    /// Requests cancellation and wakes a worker blocked on a conflict prompt.
    pub fn cancel(&self) {
        self.is_cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.active_conflict.wake();
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
            TransferJobStatus::Completed | TransferJobStatus::Failed | TransferJobStatus::Cancelled
        )
    }
}

/// Long-running batch operations shown in the unified Transfer UI.
///
/// Not every variant is a classic “byte copy”: wipe / compress / extract are
/// included so the user learns **one** progress surface (queue, minimize,
/// cancel, log) instead of a separate modal per feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TransferOperation {
    Copy,
    Move,
    Delete,
    /// Secure overwrite-then-delete (local paths).
    Wipe,
    /// Pack sources into `destination` archive (e.g. `.zip`).
    Compress,
    /// Unpack archive `sources[0]` into `destination` directory.
    Extract,
    /// Run a shell template once per source path (`shell_template`, `%f`).
    ApplyCommand,
    /// Put trashed `sources` back where they were (local only).
    Restore,
}

impl TransferOperation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Copy => "Copy",
            Self::Move => "Move",
            Self::Delete => "Delete",
            Self::Wipe => "Wipe",
            Self::Compress => "Compress",
            Self::Extract => "Extract",
            Self::ApplyCommand => "Apply",
            Self::Restore => "Restore",
        }
    }

    /// Byte-oriented path ops that use the local transfer worker scan/copy phases.
    pub fn uses_local_worker(self) -> bool {
        matches!(self, Self::Copy | Self::Move | Self::Delete)
    }

    /// Ops implemented by the wipe/archive/apply Strategy backends (local only).
    pub fn uses_ops_backend(self) -> bool {
        matches!(
            self,
            Self::Wipe | Self::Compress | Self::Extract | Self::ApplyCommand | Self::Restore
        )
    }
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
    /// Destination folders the job created (they did not exist before).
    pub created_dirs: Vec<PathBuf>,
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
    /// The destination existed and was overwritten.
    pub replaced: bool,
    /// The destination as the job left it (copy/move only).
    pub dst_stamp: Option<crate::fs::stamp::Stamp>,
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
