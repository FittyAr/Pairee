pub mod init;
pub mod lint;
pub mod package;
pub mod submit;

pub use init::{init, init_with_progress};
pub use lint::{lint, lint_with_progress};
pub use package::{package, package_to_registry_with_progress, validate_for_publish};
pub use submit::{commit_registry_changes_with_progress, run_automatic_submit, submit};

use crate::app::state::DevProgress;
use tokio::sync::mpsc::UnboundedSender;

const TEMPLATE_BRANCH: &str = "plugin-template";

/// Convenience helpers to emit `DevProgress` updates from the (synchronous)
/// backend. The sender is `Option` so that CLI callers (which have no UI to
/// drive) can pass `None` and the function no-ops.
pub(crate) fn progress_status(
    tx: &Option<UnboundedSender<DevProgress>>,
    status: impl Into<String>,
) {
    if let Some(tx) = tx {
        let _ = tx.send(DevProgress {
            status: status.into(),
            current: None,
            total: None,
            done: false,
            result: None,
            error: None,
        });
    }
}

pub(crate) fn progress_progress(
    tx: &Option<UnboundedSender<DevProgress>>,
    status: impl Into<String>,
    current: usize,
    total: usize,
) {
    if let Some(tx) = tx {
        let _ = tx.send(DevProgress {
            status: status.into(),
            current: Some(current),
            total: Some(total),
            done: false,
            result: None,
            error: None,
        });
    }
}

pub(crate) fn progress_finish(
    tx: Option<UnboundedSender<DevProgress>>,
    result: Option<String>,
    error: Option<String>,
) {
    if let Some(tx) = tx {
        let _ = tx.send(DevProgress {
            status: String::new(),
            current: None,
            total: None,
            done: true,
            result,
            error,
        });
    }
}

/// Locates the Pairee git repository on the local file system.
///
/// Search order:
/// 1. `PAIREE_REPO_DIR` environment variable (explicit override).
/// 2. Walk up from the running binary until a `.git` directory is found.
pub(crate) fn find_pairee_repo() -> Option<std::path::PathBuf> {
    // 1. Explicit env override
    if let Ok(dir) = std::env::var("PAIREE_REPO_DIR") {
        let p = std::path::PathBuf::from(dir);
        if p.join(".git").exists() {
            return Some(p);
        }
    }

    // 2. Walk up from binary location
    if let Ok(exe) = std::env::current_exe() {
        let mut candidate = exe.parent()?.to_path_buf();
        loop {
            if candidate.join(".git").exists() {
                return Some(candidate);
            }
            match candidate.parent() {
                Some(p) => candidate = p.to_path_buf(),
                None => break,
            }
        }
    }

    None
}
