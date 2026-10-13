//! Handles key input for the generic GitConfirmAction dialog.
//!
//! Every confirmed action maps to one row of [`plan`]: the Git call (run in
//! the background through `app::git_local`), the error message key and what
//! to show afterwards.

use super::remote_manage::{show_remotes, then_list_remotes};
use crate::app::context::AppContext;
use crate::app::form::confirm_answer;
use crate::app::git_local::GitContext;
use crate::app::state::popup::{Dialog, GitConfirmActionState, KeyHandler};
use crate::app::state::types::GitConfirmedAction;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;
use git2::Repository;
use std::path::Path;

/// Opens the confirmation of `action` over the current dialog.
pub fn open_confirm(
    state: &mut AppState,
    repo_path: &Path,
    message: String,
    action: GitConfirmedAction,
) {
    state.dialogs.push(GitConfirmActionState {
        message,
        repo_path: repo_path.to_path_buf(),
        action,
    });
}

pub fn handle_confirm_action(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if !matches!(state.dialogs.top(), Some(PopupType::GitConfirmAction(_))) {
        return Err(());
    }
    let Some(confirmed) = confirm_answer(&key, true) else {
        return Ok(None);
    };
    if let Some(PopupType::GitConfirmAction(confirm)) = state.dialogs.pop() {
        // Declined: the dialog under it (the Git panel) is on top again.
        if confirmed {
            run(state, confirm);
        }
    }
    Ok(None)
}

/// What to show once an action succeeded (over the refreshed Git panel).
#[derive(Clone, Copy)]
enum After {
    /// Just the refreshed panel.
    Nothing,
    /// An info popup with this message key.
    Info(&'static str),
    /// Conflict error (first key, `{}` = the conflicted files) or success
    /// info (second key), depending on whether the index has conflicts.
    ConflictsOr(&'static str, &'static str),
}

impl After {
    /// The popup to show, given the conflicted files of the index.
    fn popup(self, conflicts: &[String]) -> Option<PopupType> {
        match self {
            After::Nothing => None,
            After::Info(key) => Some(PopupType::info(t(key))),
            After::ConflictsOr(key, _) if !conflicts.is_empty() => Some(PopupType::error(
                t(key).replace("{}", &crate::app::git_ops::conflict_summary(conflicts)),
            )),
            After::ConflictsOr(_, key) => Some(PopupType::info(t(key))),
        }
    }
}

fn run(state: &mut AppState, confirm: GitConfirmActionState) {
    let GitConfirmActionState {
        repo_path, action, ..
    } = confirm;
    match action {
        GitConfirmedAction::DeleteRemoteBranch { remote, branch } => {
            // Network push: run in the background with progress.
            state.start_git_op(
                crate::app::git_ops::GitNetOp::DeleteRemoteBranch {
                    repo_path: repo_path.clone(),
                    remote,
                    branch,
                },
                crate::app::git_ops::FollowUp::RefreshPanelOnTop { repo_path },
            );
        }
        GitConfirmedAction::DeleteRemote(name) => {
            // Back to the remote list (under the confirmation), refreshed
            // once the remote is gone.
            state.run_git_local(
                &repo_path,
                move |repo| {
                    let deleted = crate::git::remote::delete_remote(repo, &name);
                    then_list_remotes(repo, deleted.ctx("git_error_delete_remote_failed"))
                },
                show_remotes,
            );
        }
        action => {
            let (exec, error_key, after) = plan(action);
            let reload = repo_path.clone();
            state.run_git_local(
                &repo_path,
                move |repo| {
                    exec(repo).ctx(error_key)?;
                    Ok(crate::git::outcome::conflicted_paths(repo).unwrap_or_default())
                },
                move |state, conflicts: Vec<String>| {
                    state.reload_git_panel(&reload);
                    if let Some(popup) = after.popup(&conflicts) {
                        state.dialogs.push(popup);
                    }
                },
            );
        }
    }
}

/// The Git call of a confirmed action (runs on the job thread).
type Exec = Box<dyn FnOnce(&mut Repository) -> anyhow::Result<()> + Send>;

/// Git call, error message key and follow-up of `action`.
fn plan(action: GitConfirmedAction) -> (Exec, &'static str, After) {
    use crate::git::{branches, cherry_pick, merge, rebase, reset, revert, stage, stash, tags};
    use GitConfirmedAction as A;
    const OK: &str = "git_operation_success";
    match action {
        A::DeleteBranch(name) => (
            Box::new(move |r| branches::delete_branch(r, &name)),
            "git_error_delete_branch_failed",
            After::Nothing,
        ),
        A::MergeBranch(name) => (
            Box::new(move |r| merge::merge(r, &name).map(|_| ())),
            "git_error_merge_failed",
            After::ConflictsOr("git_error_merge_conflicts", "git_merge_success"),
        ),
        A::RebaseBranch(onto) => (
            Box::new(move |r| rebase::rebase_branch(r, &onto).map(|_| ())),
            "git_error_rebase_failed",
            After::Info(OK),
        ),
        A::StashDrop(index) => (
            Box::new(move |r| stash::stash_drop(r, index)),
            "git_error_stash_drop_failed",
            After::Nothing,
        ),
        A::StashPop(index) => (
            Box::new(move |r| stash::stash_pop(r, index)),
            "git_error_stash_pop_failed",
            After::Nothing,
        ),
        A::StashClear => (
            Box::new(stash::stash_clear),
            "git_error_stash_clear_failed",
            After::Info(OK),
        ),
        A::ResetCommit(hash, mode) => (
            Box::new(move |r| reset::reset(r, &hash, mode)),
            "git_error_reset_failed",
            After::Nothing,
        ),
        A::DiscardFile(path) => (
            Box::new(move |r| stage::discard_file_changes(r, &path)),
            "git_error_discard_failed",
            After::Nothing,
        ),
        A::AbortMerge => (
            Box::new(|r| merge::abort_merge(r)),
            "git_error_abort_merge_failed",
            After::Nothing,
        ),
        A::CherryPick(hash) => (
            Box::new(move |r| cherry_pick::cherry_pick(r, &hash).map(|_| ())),
            "git_error_cherry_pick_failed",
            After::ConflictsOr("git_error_cherry_pick_conflicts", OK),
        ),
        A::Revert(hash) => (
            Box::new(move |r| revert::revert(r, &hash).map(|_| ())),
            "git_error_revert_failed",
            After::ConflictsOr("git_error_revert_conflicts", OK),
        ),
        A::DeleteTag(name) => (
            Box::new(move |r| tags::delete_tag(r, &name)),
            "git_error_delete_tag_failed",
            After::Info(OK),
        ),
        // Handled by `run` (network job / remote list).
        A::DeleteRemote(_) | A::DeleteRemoteBranch { .. } => {
            (Box::new(|_| Ok(())), "", After::Nothing)
        }
    }
}

impl Dialog for GitConfirmActionState {
    fn key_handler(&self) -> KeyHandler {
        handle_confirm_action
    }
}
