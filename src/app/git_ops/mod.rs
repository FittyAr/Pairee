//! Git network operations (fetch, pull, push, push tags, delete remote
//! branch, clone) executed as background jobs with progress and cancellation.
//!
//! The UI thread only validates input and builds a [`GitNetOp`]; the job runs
//! it on the blocking pool (`app::jobs`), streaming
//! [`TransferStats`](crate::git::remote::TransferStats) to the
//! `GitProgress` popup. When it finishes, [`poll_git_op`] applies the
//! UI-side continuation ([`FollowUp`]).

mod finish;
mod run;

#[cfg(test)]
mod tests;

pub use finish::poll_git_op;

use crate::app::jobs::JobSlot;
use crate::app::state::{AppState, PopupType};
use crate::git::remote::TransferStats;
use std::path::PathBuf;

/// What to run in the background. Owned data only: it crosses threads.
#[derive(Debug, Clone)]
pub enum GitNetOp {
    Fetch {
        repo_path: PathBuf,
    },
    Pull {
        repo_path: PathBuf,
        branch: String,
    },
    Push {
        repo_path: PathBuf,
        branch: String,
        set_upstream: bool,
    },
    PushTags {
        repo_path: PathBuf,
        remote: String,
    },
    DeleteRemoteBranch {
        repo_path: PathBuf,
        remote: String,
        branch: String,
    },
    Clone {
        url: String,
        target: PathBuf,
    },
}

impl GitNetOp {
    /// Localization key of the progress popup title.
    pub fn title_key(&self) -> &'static str {
        match self {
            Self::Fetch { .. } => "git_progress_fetch",
            Self::Pull { .. } => "git_progress_pull",
            Self::Push { .. } | Self::PushTags { .. } | Self::DeleteRemoteBranch { .. } => {
                "git_progress_push"
            }
            Self::Clone { .. } => "git_progress_clone",
        }
    }

    /// Localization key prefixed to the error message on failure.
    pub fn error_key(&self) -> &'static str {
        match self {
            Self::Fetch { .. } => "git_error_fetch_failed",
            Self::Pull { .. } => "git_error_pull_failed",
            Self::Push { .. } => "git_error_push_failed",
            Self::PushTags { .. } => "git_error_push_tags_failed",
            Self::DeleteRemoteBranch { .. } => "git_error_delete_remote_branch_failed",
            Self::Clone { .. } => "git_clone_error",
        }
    }
}

/// UI-side continuation applied on success (never sent to the job).
#[derive(Debug)]
pub enum FollowUp {
    /// Show the generic success message.
    Info,
    /// Reload the Git panel at this tab/cursor, then show success.
    RefreshGitPanel {
        repo_path: PathBuf,
        active_tab: usize,
        cursor_idx: usize,
    },
    /// Restore the popup that was open before (refreshing a Git panel).
    RestorePopup {
        previous: Box<PopupType>,
        repo_path: PathBuf,
    },
    /// Reread the file panels and show the clone success message.
    Cloned { show_hidden: bool },
}

/// Outcome delivered by the job: `Err` carries the error text, `None` inside
/// means the user cancelled.
pub type GitOpResult = Result<(), Option<String>>;

/// The single in-flight Git network operation (one at a time).
#[derive(Debug, Default)]
pub struct GitOpState {
    pub job: JobSlot<GitOpResult, TransferStats>,
    pending: Option<(GitNetOp, FollowUp)>,
}

impl GitOpState {
    pub fn is_running(&self) -> bool {
        self.job.is_running()
    }

    /// Latest transfer statistics of the running operation.
    pub fn progress(&self) -> Option<TransferStats> {
        self.job.progress()
    }
}

impl AppState {
    /// Starts `op` in the background and shows the progress popup on top of
    /// the current dialogs. A running operation is never superseded.
    pub fn start_git_op(&mut self, op: GitNetOp, follow_up: FollowUp) {
        if self.git_op.is_running() {
            self.dialogs
                .push(PopupType::Info(crate::config::localization::t(
                    "git_operation_busy",
                )));
            return;
        }
        let title = crate::config::localization::t(op.title_key());
        let job_op = op.clone();
        self.git_op.pending = Some((op, follow_up));
        self.dialogs.push(PopupType::GitProgress { title });
        self.git_op.job.start(move |ctx| run::run(&job_op, ctx));
        // Inline execution (no runtime) has finished already.
        poll_git_op(self);
    }

    /// Cancels the running operation (Esc in the progress popup).
    pub fn cancel_git_op(&mut self) {
        if !self.git_op.is_running() {
            return;
        }
        self.git_op.job.cancel();
        self.git_op.pending = None;
        if matches!(self.dialogs.top(), Some(PopupType::GitProgress { .. })) {
            self.dialogs.pop();
        }
        self.dialogs
            .push(PopupType::Info(crate::config::localization::t(
                "git_operation_cancelled",
            )));
    }
}
