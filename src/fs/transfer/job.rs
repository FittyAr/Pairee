use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use super::endpoint::TransferEndpoint;

#[derive(Debug, Clone)]
pub struct TransferJob {
    pub id: Uuid,
    pub operation: TransferOperation,
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    /// Endpoint that produces the source files. For `Delete`, this is
    /// the same as `dst_endpoint`. For `Copy` / `Move`, this is the
    /// panel the user is reading from.
    pub src_endpoint: TransferEndpoint,
    /// Endpoint that consumes the destination. For `Delete` this is
    /// unused (and defaults to `Local` for back-compat). For
    /// `Copy` / `Move` this is the panel the user is writing to.
    pub dst_endpoint: TransferEndpoint,
    pub options: super::options::TransferOptions,
    pub status: TransferJobStatus,
    pub results: TransferResults,
    pub progress: Option<TransferProgress>,
    pub log_lines: Vec<String>,
    pub is_paused: Arc<std::sync::atomic::AtomicBool>,
    pub is_cancelled: Arc<std::sync::atomic::AtomicBool>,
    pub skip_file_flag: Arc<std::sync::atomic::AtomicBool>,
    pub active_conflict: Arc<std::sync::Mutex<Option<super::conflict::ConflictResolution>>>,
}

impl TransferJob {
    pub fn new(
        operation: TransferOperation,
        sources: Vec<PathBuf>,
        destination: PathBuf,
        options: super::options::TransferOptions,
    ) -> Self {
        Self::with_endpoints(
            operation,
            sources,
            destination,
            options,
            TransferEndpoint::Local,
            TransferEndpoint::Local,
        )
    }

    /// Constructor that takes explicit source / destination
    /// endpoints. Use this from Phase 5 onwards when the action
    /// handlers can read the panel configuration and pick the right
    /// endpoint per side.
    pub fn with_endpoints(
        operation: TransferOperation,
        sources: Vec<PathBuf>,
        destination: PathBuf,
        options: super::options::TransferOptions,
        src_endpoint: TransferEndpoint,
        dst_endpoint: TransferEndpoint,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            operation,
            sources,
            destination,
            src_endpoint,
            dst_endpoint,
            options,
            status: TransferJobStatus::Queued,
            results: TransferResults::default(),
            progress: None,
            log_lines: Vec::new(),
            is_paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            is_cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            skip_file_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
            TransferJobStatus::Completed | TransferJobStatus::Failed | TransferJobStatus::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LinkKind {
    Symbolic,
    Hard,
}

/// Archive container format used by the
/// `TransferOperation::Compress` and
/// `TransferOperation::Extract` variants. The encoding
/// is the same on both sides: a `Compress { format:
/// ArchiveFormat::Zip, .. }` produces a file that
/// `Extract { format: ArchiveFormat::Zip }` can read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ArchiveFormat {
    /// `.zip` with DEFLATE compression (level 0..=9,
    /// where 0 = "store only" / no compression).
    Zip,
    /// `.tar.gz` (gzip-compressed tar). The level
    /// applies to the gzip layer; tar itself does not
    /// compress. Also used for plain `.tar` files:
    /// the pipeline tries the gzip layer first and
    /// surfaces a clear error if the stream is not
    /// gzipped (we don't yet support raw tar without
    /// a wrapper; that is a future ticket).
    TarGz,
    /// `.7z` (LZMA). `level` is the LZMA compression
    /// level 0..=9.
    SevenZ,
}

impl ArchiveFormat {
    /// Detect the archive format from a file path's
    /// extension. Returns `None` when the extension is
    /// not recognised.
    ///
    /// Recognised extensions:
    /// * `.zip`  → `Zip`
    /// * `.gz`, `.tgz`, `.tar` → `TarGz`
    /// * `.7z`   → `SevenZ`
    ///
    /// Both call sites that used to inline this match
    /// (`actions/fs_ops/extract.rs` and
    /// `input_popup/archive_commands.rs`) now go through
    /// here so the rules stay in sync.
    pub fn detect_from_path(path: &std::path::Path) -> Option<Self> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())?
            .to_ascii_lowercase();
        // Some archives carry a double extension
        // (`.tar.gz`, `.tar.bz2`, ...). `Path::extension`
        // only sees the last one, so peek at the
        // file_name when the last extension is `.gz` /
        // `.bz2` / `.xz` and try to recognise the
        // combined form first.
        if ext == "gz" || ext == "tgz" {
            return Some(Self::TarGz);
        }
        if let Some(stem_ext) = path
            .file_stem()
            .and_then(|s| std::path::Path::new(s).extension())
            .and_then(|e| e.to_str())
        {
            let combined = format!("{}.{}", stem_ext, ext).to_ascii_lowercase();
            if combined == "tar.gz" || combined == "tar.tgz" {
                return Some(Self::TarGz);
            }
        }
        match ext.as_str() {
            "zip" => Some(Self::Zip),
            "7z" => Some(Self::SevenZ),
            "tar" => Some(Self::TarGz),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TransferOperation {
    Copy,
    Move,
    Delete,
    /// Single-source single-destination rename. The
    /// `sources` vector must contain exactly one entry, and
    /// `destination` is the new name (not a parent
    /// directory). Both endpoints are the same: rename across
    /// filesystems / panels is rejected by the engine.
    Rename,
    /// Create a symbolic or hard link at `destination` pointing
    /// to the single entry in `sources`. Both endpoints are the
    /// same (link across endpoints is rejected). Hard links
    /// over SSH are rejected with a clear error (SFTP v3 has
    /// no `link` command and we opted out of shell-out).
    CreateLink {
        kind: LinkKind,
    },
    /// Bundle `sources` into a single archive at
    /// `destination`. `format` drives the encoder
    /// (zip / tar.gz / 7z); `level` is the compression
    /// level 0..=9 where 0 means "store only" (zip
    /// only — tar/7z treat 0 as a low level). For
    /// `Compress`, `destination` is the path of the
    /// archive itself (e.g. `backup.zip`); the engine
    /// creates parents as needed. SSH endpoints are
    /// supported: readers go through the source
    /// endpoint, the writer goes through the
    /// destination endpoint.
    Compress {
        format: ArchiveFormat,
        level: u8,
    },
    /// Reverse of `Compress`: read a single archive
    /// from `sources[0]` and unpack it into
    /// `destination` (a directory). `format` is
    /// required because the engine does not
    /// auto-detect format from the file extension
    /// (the caller / UI already knows the format
    /// from the popup). Path-traversal entries
    /// (`../`, absolute paths, NUL bytes) are
    /// rejected by the extract pipeline.
    Extract {
        format: ArchiveFormat,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_format_serialises_round_trip() {
        for f in [
            ArchiveFormat::Zip,
            ArchiveFormat::TarGz,
            ArchiveFormat::SevenZ,
        ] {
            let json = serde_json::to_string(&f).unwrap();
            let back: ArchiveFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(f, back);
        }
    }

    /// Centralised extension detection (security review
    /// finding L6). The two call sites used to inline
    /// this match and silently fall back to Zip for
    /// unknown extensions including `.tar`; we now go
    /// through this helper which also recognises `.tar`
    /// and rejects unknown extensions explicitly.
    #[test]
    fn archive_format_detect_from_path() {
        use std::path::Path;
        let cases: &[(&str, Option<ArchiveFormat>)] = &[
            ("foo.zip", Some(ArchiveFormat::Zip)),
            ("FOO.ZIP", Some(ArchiveFormat::Zip)),
            ("foo.tar.gz", Some(ArchiveFormat::TarGz)),
            ("foo.tgz", Some(ArchiveFormat::TarGz)),
            ("foo.tar", Some(ArchiveFormat::TarGz)),
            ("foo.7z", Some(ArchiveFormat::SevenZ)),
            // Unknown / unsupported extensions
            ("foo.rar", None),
            ("foo.iso", None),
            ("foo", None),
            // No extension
            (".", None),
        ];
        for (input, expected) in cases {
            let got = ArchiveFormat::detect_from_path(Path::new(input));
            assert_eq!(
                got, *expected,
                "detect_from_path({input:?}) returned {got:?}, expected {expected:?}",
            );
        }
    }

    #[test]
    fn transfer_operation_serde_round_trip_for_all_variants() {
        let ops = vec![
            TransferOperation::Copy,
            TransferOperation::Move,
            TransferOperation::Delete,
            TransferOperation::Rename,
            TransferOperation::CreateLink {
                kind: LinkKind::Hard,
            },
            TransferOperation::Compress {
                format: ArchiveFormat::Zip,
                level: 6,
            },
            TransferOperation::Extract {
                format: ArchiveFormat::TarGz,
            },
        ];
        for op in ops {
            let json = serde_json::to_string(&op).unwrap();
            let back: TransferOperation = serde_json::from_str(&json).unwrap();
            assert_eq!(op, back, "round-trip failed for {:?}", op);
        }
    }

    #[test]
    fn transfer_job_constructor_accepts_compress() {
        // The TransferJob::new constructor doesn't
        // care about the operation kind; the variant
        // is just data. This is a smoke test that
        // the new variants can ride through the
        // same constructor as the older ones.
        let job = TransferJob::new(
            TransferOperation::Compress {
                format: ArchiveFormat::SevenZ,
                level: 9,
            },
            vec![PathBuf::from("/src/folder")],
            PathBuf::from("/dest/archive.7z"),
            super::super::options::TransferOptions::default(),
        );
        assert_eq!(
            job.operation,
            TransferOperation::Compress {
                format: ArchiveFormat::SevenZ,
                level: 9,
            }
        );
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
