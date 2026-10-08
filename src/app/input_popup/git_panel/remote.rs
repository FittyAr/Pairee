//! Git remote operations (fetch, pull, push) for git_panel.
//!
//! Only cheap local checks run here; the network work is a background job
//! (`app::git_ops`) with a progress popup and Esc to cancel.

use crate::app::git_ops::{FollowUp, GitNetOp};
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use std::path::Path;

pub fn handle_fetch(state: &mut AppState, repo_path: &Path) {
    state.start_git_op(
        GitNetOp::Fetch {
            repo_path: repo_path.to_path_buf(),
        },
        FollowUp::Info,
    );
}

pub fn handle_pull(state: &mut AppState, repo_path: &Path, active_tab: usize, cursor_idx: usize) {
    if let Some((_, branch)) = checked_out_branch(state, repo_path) {
        state.start_git_op(
            GitNetOp::Pull {
                repo_path: repo_path.to_path_buf(),
                branch,
            },
            FollowUp::RefreshGitPanel {
                repo_path: repo_path.to_path_buf(),
                active_tab,
                cursor_idx,
            },
        );
    }
}

pub fn handle_push(state: &mut AppState, repo_path: &Path) {
    if let Some((repo, branch)) = checked_out_branch(state, repo_path) {
        let set_upstream = repo
            .find_branch(&branch, git2::BranchType::Local)
            .and_then(|b| b.upstream())
            .is_err();
        state.start_git_op(
            GitNetOp::Push {
                repo_path: repo_path.to_path_buf(),
                branch,
                set_upstream,
            },
            FollowUp::Info,
        );
    }
}

/// Repository plus checked-out branch name; shows an error and returns
/// `None` when HEAD is detached (pull/push need a branch).
fn checked_out_branch(
    state: &mut AppState,
    repo_path: &Path,
) -> Option<(git2::Repository, String)> {
    let repo = crate::git::repo::find_repo(repo_path)?;
    let branch = if repo.head_detached().unwrap_or(false) {
        None
    } else {
        repo.head()
            .ok()
            .and_then(|h| h.shorthand().ok().map(|s| s.to_string()))
    };
    match branch {
        Some(branch) => Some((repo, branch)),
        None => {
            state
                .dialogs
                .replace(PopupType::Error(t("git_error_detached_head_operation")));
            None
        }
    }
}
