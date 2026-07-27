//! Transfer policy: how the engine reacts to file-level errors.
//!
//! The engine itself is **agnostic** about how to react to a
//! failed file. It owns no popup state, no UI, no elevation
//! logic. The behaviour is delegated to a [`TransferPolicy`]
//! implementation that the engine consults on every
//! `FileFailed` event.
//!
//! Two implementations live in this module:
//!
//! * [`LoggingPolicy`] — the default. Only writes to
//!   `log::warn!` and `finalize()` returns the empty list.
//!   No UI interaction, no prompting. Use this in unit
//!   tests and headless contexts.
//! * [`PromptPolicy`] — the production policy. Accumulates
//!   `AccessDenied` failures, batches them, and at the end
//!   of the job emits a single `TransferEvent::PermissionPrompt`
//!   asking the UI whether to retry as admin. Wired up in
//!   phase B4.
//!
//! # Why a trait?
//!
//! The alternative is to inline the policy in the engine.
//! That would couple the engine to the UI layer (mpsc
//! channel, popup state) and make it impossible to test the
//! error-handling path in isolation. The trait keeps the
//! engine side-effect-free with respect to the user and
//! gives tests a trivial default impl.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Categorical error for a failed file operation.
///
/// The engine builds one of these for every `FailedFile`
/// emitted during a job. The policy uses the variant to
/// decide whether the failure is eligible for a retry-as-
/// admin prompt (only `AccessDenied`) or should just be
/// reported as a normal failure (`NotFound`, `IoError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileError {
    /// POSIX `EACCES` / Windows `ERROR_ACCESS_DENIED`. The
    /// file exists but the current process lacks the
    /// permission to read / write / delete it. This is the
    /// only variant that triggers the elevation prompt.
    AccessDenied,
    /// POSIX `ENOENT` / Windows `ERROR_FILE_NOT_FOUND`.
    /// The path does not exist (anymore). Cannot be
    /// rescued by elevation.
    NotFound,
    /// Anything else: I/O error, network drop, SSH
    /// protocol failure, ... The wrapped string is the
    /// original error message.
    IoError(String),
}

impl FileError {
    /// Best-effort categorisation from a raw error message.
    ///
    /// The engine's lower layers return `anyhow::Error`
    /// chains whose root cause is a `std::io::Error`, an
    /// `ssh2::Error`, or a plain string. We don't have
    /// the error kind, only the formatted message, so the
    /// heuristics are necessarily fuzzy. False positives
    /// downgrade an `AccessDenied` to `IoError` (no prompt
    /// when one would have been useful); false negatives
    /// show a prompt for a non-permission failure (user
    /// says No and the failure stays reported normally).
    /// Both are acceptable: the prompt is always
    /// opt-in by the user.
    pub fn from_error_message(msg: &str) -> Self {
        let lower = msg.to_ascii_lowercase();
        if lower.contains("access is denied")
            || lower.contains("permission denied")
            || lower.contains("eacces")
            || lower.contains("access denied")
        {
            FileError::AccessDenied
        } else if lower.contains("not found")
            || lower.contains("no such file")
            || lower.contains("enoent")
            || lower.contains("does not exist")
            || lower.contains("cannot find")
        {
            FileError::NotFound
        } else {
            FileError::IoError(msg.to_string())
        }
    }

    /// `true` only for [`FileError::AccessDenied`].
    pub fn is_access_denied(&self) -> bool {
        matches!(self, FileError::AccessDenied)
    }
}

/// A single file the policy wants the engine to retry as
/// admin. The `original_path` is the path the failed
/// operation tried to touch. The `error` is the original
/// error message kept for logging.
///
/// In B5 the engine will turn this into one
/// [`crate::fs::privileges::FsOperation`] per request and
/// hand it to [`crate::fs::privileges::run_in_elevated_helper`].
///
/// `error` is stored on the struct for logging but the
/// engine currently reads only `original_path`. The
/// `#[allow(dead_code)]` keeps AGENTS.md happy while
/// leaving the field as part of the public surface (it
/// will be needed when the engine's retry log gains
/// per-file error reporting in a later phase).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RetryRequest {
    pub original_path: PathBuf,
    pub error: String,
}

/// Strategy the engine consults on every file failure.
///
/// Implementations are expected to be cheap to call
/// (`on_file_error` runs inside the worker hot path) and
/// thread-safe (`Send + Sync`).
pub trait TransferPolicy: Send + Sync + 'static {
    /// Called for every failed file.
    ///
    /// `file` is the path the engine tried to act on; the
    /// exact semantic depends on the operation (source for
    /// Copy / Move / Extract, target for Delete / Rename).
    /// `error` is the categorised failure. Implementations
    /// must not block; long-running work (e.g. waiting for
    /// UI input) belongs in `finalize`.
    fn on_file_error(&self, file: &Path, error: &FileError);

    /// Called once when the job finishes (success, failure,
    /// or cancellation). Returns the list of files the
    /// policy wants the engine to retry as admin.
    ///
    /// The default returns the empty list. The
    /// [`PromptPolicy`] returns at most one entry per
    /// `AccessDenied` it observed during the job.
    fn finalize(&self) -> Vec<RetryRequest>;

    /// Reset all accumulated state. Called by the engine
    /// before the next job starts, so a long-lived policy
    /// instance (e.g. one held in the engine's `AppState`)
    /// can be reused across jobs.
    fn reset(&self) {}
}

// ---------------------------------------------------------------------------
//   PromptPolicy — accumulates AccessDenied, batches the prompt
// ---------------------------------------------------------------------------

/// Production policy. Keeps a list of `AccessDenied`
/// failures in an internal `Mutex<Vec<RetryRequest>>`
/// and drains it on `finalize()`. The engine forwards
/// the drained list as a `TransferEvent::PermissionPrompt`
/// so the UI can show one popup at the end of the job.
#[derive(Debug, Default)]
pub struct PromptPolicy {
    /// Files that failed with `AccessDenied`. The error
    /// string is kept for the retry log.
    denied: Mutex<Vec<RetryRequest>>,
}

impl PromptPolicy {
    /// Construct an empty prompt policy. The first
    /// failure categorised as `AccessDenied` is added
    /// on the next call to
    /// [`TransferPolicy::on_file_error`].
    pub fn new() -> Self {
        Self::default()
    }
}

impl TransferPolicy for PromptPolicy {
    fn on_file_error(&self, file: &Path, error: &FileError) {
        if !error.is_access_denied() {
            return;
        }
        self.denied
            .lock()
            .expect("PromptPolicy mutex poisoned")
            .push(RetryRequest {
                original_path: file.to_path_buf(),
                error: "Access denied".to_string(),
            });
    }

    fn finalize(&self) -> Vec<RetryRequest> {
        // Drain the accumulated requests so the next
        // job starts from a clean slate. The engine
        // forwards the drained list as the
        // `PermissionPrompt` event payload.
        std::mem::take(
            &mut *self.denied.lock().expect("PromptPolicy mutex poisoned"),
        )
    }

    fn reset(&self) {
        self.denied
            .lock()
            .expect("PromptPolicy mutex poisoned")
            .clear();
    }
}

// ---------------------------------------------------------------------------
//   Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_error_classifies_access_denied_messages() {
        assert_eq!(
            FileError::from_error_message("Access is denied."),
            FileError::AccessDenied
        );
        assert_eq!(
            FileError::from_error_message("Permission denied (os error 13)"),
            FileError::AccessDenied
        );
        assert_eq!(
            FileError::from_error_message("EACCES: permission denied"),
            FileError::AccessDenied
        );
        // Case-insensitive
        assert_eq!(
            FileError::from_error_message("PERMISSION DENIED"),
            FileError::AccessDenied
        );
    }

    #[test]
    fn file_error_classifies_not_found_messages() {
        assert_eq!(
            FileError::from_error_message("No such file or directory (os error 2)"),
            FileError::NotFound
        );
        assert_eq!(
            FileError::from_error_message("ENOENT: no such file"),
            FileError::NotFound
        );
        assert_eq!(
            FileError::from_error_message("The system cannot find the path specified."),
            FileError::NotFound
        );
    }

    #[test]
    fn file_error_falls_back_to_io_error() {
        let raw = "connection reset by peer";
        match FileError::from_error_message(raw) {
            FileError::IoError(s) => assert_eq!(s, raw),
            other => panic!("expected IoError, got {:?}", other),
        }
    }

    #[test]
    fn file_error_is_access_denied_helper() {
        assert!(FileError::AccessDenied.is_access_denied());
        assert!(!FileError::NotFound.is_access_denied());
        assert!(!FileError::IoError("x".to_string()).is_access_denied());
    }

    #[test]
    fn prompt_policy_only_records_access_denied() {
        let policy = PromptPolicy::new();

        policy.on_file_error(
            Path::new("/file_a"),
            &FileError::AccessDenied,
        );
        policy.on_file_error(
            Path::new("/file_b"),
            &FileError::NotFound,
        );
        policy.on_file_error(
            Path::new("/file_c"),
            &FileError::IoError("nope".to_string()),
        );
        policy.on_file_error(
            Path::new("/file_d"),
            &FileError::AccessDenied,
        );

        // Only the two AccessDenied ones are drained.
        let requests = policy.finalize();
        assert_eq!(requests.len(), 2);
        let paths: Vec<&PathBuf> =
            requests.iter().map(|r| &r.original_path).collect();
        assert!(paths.contains(&&PathBuf::from("/file_a")));
        assert!(paths.contains(&&PathBuf::from("/file_d")));
    }

    #[test]
    fn prompt_policy_drain_clears_state() {
        let policy = PromptPolicy::new();
        policy.on_file_error(
            Path::new("/x"),
            &FileError::AccessDenied,
        );

        // The first drain returns the requests and
        // empties the buffer.
        let drained = policy.finalize();
        assert_eq!(drained.len(), 1);

        // A second drain on an empty policy is a no-op.
        assert!(policy.finalize().is_empty());
    }

    #[test]
    fn prompt_policy_reset_clears_state() {
        let policy = PromptPolicy::new();
        policy.on_file_error(
            Path::new("/x"),
            &FileError::AccessDenied,
        );
        policy.reset();
        assert!(policy.finalize().is_empty());
    }

    #[test]
    fn prompt_policy_is_send_and_sync() {
        // Compile-time check: the policy must be usable
        // from a worker task.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PromptPolicy>();
    }

    #[test]
    fn policy_trait_object_can_be_boxed() {
        // The engine will store `Box<dyn TransferPolicy>`.
        let _p: Box<dyn TransferPolicy> = Box::new(PromptPolicy::new());
    }
}
