pub mod init;
pub mod lint;
pub mod package;
pub mod submit;

pub use init::{init, init_with_progress_in};
pub use lint::lint_with_progress;
pub use package::{package_to_registry, package_to_registry_with_progress, validate_for_publish};
pub use submit::{commit_registry_changes_with_progress, run_automatic_submit};

use crate::app::state::DevProgress;
use tokio::sync::mpsc::UnboundedSender;

const TEMPLATE_BRANCH: &str = "plugin-template";

/// `Ok` when a plugin name or author is a registry identifier
/// (`[A-Za-z0-9_-]`), which is safe as a path component.
fn check_identifier(value: &str) -> anyhow::Result<()> {
    if crate::plugin::updater::is_identifier(value) {
        return Ok(());
    }
    anyhow::bail!(
        crate::config::localization::t("plugin_dev_err_invalid_identifier").replace("{}", value)
    )
}

/// The local clone of the plugin registry used to package and submit.
pub fn temp_registry_dir() -> std::path::PathBuf {
    crate::config::paths::get_cache_dir().join("temp_registry")
}

/// Folder of the plugin described by `manifest` inside the registry clone
/// `temp_dir`; its name and author must be registry identifiers (they
/// become path components, and the installer refuses anything else).
pub fn registry_plugin_dir(
    temp_dir: &std::path::Path,
    manifest: &crate::plugin::loader::PluginManifest,
) -> anyhow::Result<std::path::PathBuf> {
    use crate::plugin::updater::registry::{plugin_dir_path, registry_author};
    let author = registry_author(manifest.author.as_deref());
    check_identifier(&manifest.name)?;
    check_identifier(author)?;
    Ok(temp_dir
        .join("registry")
        .join(plugin_dir_path(author, &manifest.name)))
}

#[cfg(test)]
mod tests;

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

/// Locates a local clone of the Pairee repository that has `branch`.
///
/// Search order:
/// 1. `PAIREE_REPO_DIR` environment variable (explicit override).
/// 2. Walk up from the running binary to a folder that is a Pairee
///    repository.
///
/// A folder only counts when one of its remotes is the Pairee repository
/// and it has `branch` (locally or fetched): the first `.git` found while
/// walking up may be the user's home or an unrelated project, which must
/// never be used as the source of a template or the registry. `None` makes
/// the callers fall back to GitHub.
pub(crate) fn find_pairee_repo(branch: &str) -> Option<std::path::PathBuf> {
    // 1. Explicit env override
    if let Ok(dir) = std::env::var("PAIREE_REPO_DIR") {
        let p = std::path::PathBuf::from(dir);
        if is_pairee_repo(&p, branch) {
            return Some(p);
        }
    }

    // 2. Walk up from binary location
    let exe = std::env::current_exe().ok()?;
    exe.parent()?
        .ancestors()
        .find(|candidate| is_pairee_repo(candidate, branch))
        .map(std::path::Path::to_path_buf)
}

/// `true` when `dir` is a git repository with a Pairee remote and `branch`.
fn is_pairee_repo(dir: &std::path::Path, branch: &str) -> bool {
    let Ok(repo) = git2::Repository::open(dir) else {
        return false;
    };
    let has_branch = repo.find_branch(branch, git2::BranchType::Local).is_ok()
        || repo
            .find_reference(&format!("refs/remotes/origin/{branch}"))
            .is_ok();
    has_branch && has_pairee_remote(&repo)
}

fn has_pairee_remote(repo: &git2::Repository) -> bool {
    let Ok(names) = repo.remotes() else {
        return false;
    };
    names.iter().flatten().flatten().any(|name| {
        repo.find_remote(name)
            .ok()
            .and_then(|remote| remote.url().ok().map(str::to_string))
            .is_some_and(|url| is_pairee_url(&url))
    })
}

/// A remote URL of the Pairee repository (`https://github.com/FittyAr/Pairee`,
/// with or without `.git`, or the `git@github.com:` form).
fn is_pairee_url(url: &str) -> bool {
    let url = url.trim().trim_end_matches('/').to_ascii_lowercase();
    let url = url.strip_suffix(".git").unwrap_or(&url);
    url.ends_with("github.com/fittyar/pairee") || url.ends_with("github.com:fittyar/pairee")
}
