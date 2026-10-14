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
    /// Re-read the Git panel the operation was started over.
    RefreshPanelOnTop { repo_path: PathBuf },
    /// Reread the file panels and show the clone success message.
    Cloned { show_hidden: bool },
}

/// Outcome delivered by the job: `Ok` carries the files a pull left
/// conflicted (empty when clean); `Err` carries the error text, `None`
/// inside means the user cancelled.
pub type GitOpResult = Result<Vec<String>, Option<String>>;

/// The first conflicted `paths` on one line, "and N more" for the rest.
pub fn conflict_summary(paths: &[String]) -> String {
    const SHOWN: usize = 5;
    let mut text = paths
        .iter()
        .take(SHOWN)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    if paths.len() > SHOWN {
        text.push(' ');
        text.push_str(
            &crate::config::localization::t("git_conflicts_more")
                .replace("{}", &(paths.len() - SHOWN).to_string()),
        );
    }
    text
}

/// The single in-flight Git network operation (one at a time).
#[derive(Debug, Default)]
pub struct GitOpState {
    pub job: JobSlot<GitOpResult, TransferStats>,
    pending: Option<(GitNetOp, FollowUp)>,
    /// The user asked to cancel; the job's real outcome is still awaited.
    cancelling: bool,
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
                .push(PopupType::info(crate::config::localization::t(
                    "git_operation_busy",
                )));
            return;
        }
        let title = crate::config::localization::t(op.title_key());
        let job_op = op.clone();
        let asker = self.credential_requests.asker();
        self.git_op.pending = Some((op, follow_up));
        self.git_op.cancelling = false;
        self.dialogs
            .push(crate::app::state::popup::GitProgress { title });
        self.git_op
            .job
            .start(move |ctx| run::run(&job_op, ctx, &asker));
        // Inline execution (no runtime) has finished already.
        poll_git_op(self);
    }

    /// Cancels the running operation (Esc in the progress popup).
    ///
    /// The job is asked to stop and its outcome is still reported when it
    /// ends: an operation that got past its last cancellation point (a
    /// pull that merged) is shown as done, not as cancelled. A second Esc
    /// hides the progress popup; the outcome is still shown later.
    pub fn cancel_git_op(&mut self) {
        if !self.git_op.is_running() {
            return;
        }
        let on_top = matches!(self.dialogs.top(), Some(PopupType::GitProgress(_)));
        if self.git_op.cancelling {
            if on_top {
                self.dialogs.pop();
            }
            return;
        }
        self.git_op.job.request_cancel();
        self.git_op.cancelling = true;
        if let Some(PopupType::GitProgress(progress)) = self.dialogs.top_mut() {
            progress.title = crate::config::localization::t("git_operation_cancelling");
        }
    }
}
