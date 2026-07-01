pub mod init;
pub mod lint;
pub mod package;
pub mod submit;

pub use init::init;
pub use lint::lint;
pub use package::{package, package_to_registry, validate_for_publish};
pub use submit::{commit_registry_changes, run_automatic_submit, submit};

const TEMPLATE_BRANCH: &str = "plugin-template";

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
