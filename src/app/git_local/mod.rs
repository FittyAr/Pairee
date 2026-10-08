//! Local Git operations of the Git panel (stage, unstage, discard, diff,
//! commit, stash, branch, tag, checkout, merge, …) as background jobs.
//!
//! Every operation is a pair (Command pattern):
//!
//! * `work` runs on the blocking pool (`app::jobs`) with the repository
//!   opened there and returns a value or a localized [`GitFailure`];
//! * `done` runs on the UI thread with that value (usually: restore the Git
//!   panel and re-read it through the async loader, or open a dialog).
//!
//! [`AppState::run_git_local`] is the single entry point, so no handler
//! touches `git2` on the key path. While a job runs the Git panel title shows
//! a busy marker and new operations are refused; errors open the localized
//! alert over the current dialogs.

#[cfg(test)]
mod tests;

use crate::app::jobs::JobSlot;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use std::path::{Path, PathBuf};

/// A failed operation, already formatted as "<localized prefix>: <error>".
#[derive(Debug)]
pub struct GitFailure(String);

/// Attaches the localized error prefix `key` to a failing Git call.
pub trait GitContext<T> {
    fn ctx(self, key: &'static str) -> Result<T, GitFailure>;
}

impl<T, E: std::fmt::Display> GitContext<T> for Result<T, E> {
    fn ctx(self, key: &'static str) -> Result<T, GitFailure> {
        self.map_err(|e| GitFailure(format!("{}: {}", t(key), e)))
    }
}

/// UI-thread continuation built by the job once `work` succeeded.
type Apply = Box<dyn FnOnce(&mut AppState) + Send>;

/// The single in-flight local Git operation.
#[derive(Debug, Default)]
pub struct GitLocalJob {
    slot: JobSlot<Result<Apply, String>>,
    repo: Option<PathBuf>,
}

impl GitLocalJob {
    pub fn is_running(&self) -> bool {
        self.slot.is_running()
    }

    /// `true` while an operation on `repo_path` is running.
    pub fn is_running_for(&self, repo_path: &Path) -> bool {
        self.is_running() && self.repo.as_deref() == Some(repo_path)
    }
}

impl AppState {
    /// Runs `work` on the repository at `repo_path` in the background, then
    /// `done` with its value on the UI thread. Refused (with a notice) while
    /// another local Git operation is running.
    pub fn run_git_local<T, W, D>(&mut self, repo_path: &Path, work: W, done: D)
    where
        T: Send + 'static,
        W: FnOnce(&mut git2::Repository) -> Result<T, GitFailure> + Send + 'static,
        D: FnOnce(&mut AppState, T) + Send + 'static,
    {
        let job = &mut self.git_panel.local;
        if job.is_running() {
            self.dialogs.push(PopupType::Info(t("git_local_busy")));
            return;
        }
        let path = repo_path.to_path_buf();
        job.repo = Some(path.clone());
        job.slot.start(move |_| {
            let mut repo = crate::git::repo::find_repo(&path).ok_or_else(|| t("git_not_a_repo"))?;
            let value = work(&mut repo).map_err(|GitFailure(msg)| msg)?;
            Ok(Box::new(move |state: &mut AppState| done(state, value)) as Apply)
        });
        self.mark_ui_dirty();
        // Inline execution (no runtime) has finished already.
        poll_git_local(self);
    }
}

/// Applies a finished local Git operation. Returns `true` when one finished.
pub fn poll_git_local(state: &mut AppState) -> bool {
    let Some(outcome) = state.git_panel.local.slot.poll() else {
        return false;
    };
    state.git_panel.local.repo = None;
    match outcome {
        Ok(apply) => apply(state),
        Err(msg) => state.dialogs.push(PopupType::Error(msg)),
    }
    state.mark_ui_dirty();
    true
}

/// `done` continuation: re-read the Git panel of `repo_path` (if open).
pub fn reload_panel<T>(repo_path: &Path) -> impl FnOnce(&mut AppState, T) + Send + 'static {
    let repo_path = repo_path.to_path_buf();
    move |state, _| state.reload_git_panel(&repo_path)
}
