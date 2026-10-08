//! Handles key input for the generic GitConfirmAction dialog.
//!
//! Every confirmed action maps to one row of [`plan`]: the Git call, the
//! error message key and what to show afterwards.

use super::common::restore_previous_and_refresh;
use crate::app::context::AppContext;
use crate::app::form::confirm_answer;
use crate::app::state::popup::{GitConfirmActionState, GitPromptPopup};
use crate::app::state::types::GitConfirmedAction;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;
use git2::Repository;

pub fn handle_confirm_action(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if !matches!(
        state.dialogs.top(),
        Some(PopupType::GitPrompt(GitPromptPopup::ConfirmAction(_)))
    ) {
        return Err(());
    }
    let Some(confirmed) = confirm_answer(&key, true) else {
        return Ok(None);
    };
    if let Some(PopupType::GitPrompt(GitPromptPopup::ConfirmAction(confirm))) = state.dialogs.pop()
    {
        if confirmed {
            run(state, confirm);
        } else {
            state.dialogs.replace(*confirm.previous_popup);
        }
    }
    Ok(None)
}

/// What to show once an action succeeded (after restoring the Git panel).
enum After {
    /// Just the refreshed panel.
    Nothing,
    /// An info popup with this message key.
    Info(&'static str),
    /// Conflict error (first key) or success info (second key), depending on
    /// whether the index has conflicts.
    ConflictsOr(&'static str, &'static str),
}

fn run(state: &mut AppState, confirm: GitConfirmActionState) {
    if let GitConfirmedAction::DeleteRemoteBranch { remote, branch } = confirm.action {
        // Network push: run in the background with progress.
        state.start_git_op(
            crate::app::git_ops::GitNetOp::DeleteRemoteBranch {
                repo_path: confirm.repo_path.clone(),
                remote,
                branch,
            },
            crate::app::git_ops::FollowUp::RestorePopup {
                previous: confirm.previous_popup,
                repo_path: confirm.repo_path,
            },
        );
        return;
    }
    let Some(mut repo) = crate::git::repo::find_repo(&confirm.repo_path) else {
        // Keep the confirmation open when the repository is gone.
        state
            .dialogs
            .push(PopupType::GitPrompt(GitPromptPopup::ConfirmAction(confirm)));
        return;
    };
    let GitConfirmActionState {
        repo_path,
        action,
        previous_popup,
        ..
    } = confirm;
    let (result, error_key, after) = plan(&mut repo, &action);
    if let Err(e) = result {
        state
            .dialogs
            .replace(PopupType::Error(format!("{}: {}", t(error_key), e)));
        return;
    }
    if let GitConfirmedAction::DeleteRemote(_) = action
        && let PopupType::GitPrompt(GitPromptPopup::RemoteManage(mut manage)) = *previous_popup
    {
        manage.remotes = crate::git::remote::list_remotes(&repo).unwrap_or_default();
        if !manage.remotes.is_empty() {
            manage.selected_idx = manage.selected_idx.min(manage.remotes.len() - 1);
        }
        state
            .dialogs
            .replace(PopupType::GitPrompt(GitPromptPopup::RemoteManage(manage)));
        return;
    }
    restore_previous_and_refresh(state, *previous_popup, &repo_path);
    let has_conflicts = || repo.index().map(|i| i.has_conflicts()).unwrap_or(false);
    match after {
        After::Nothing => {}
        After::Info(key) => state.dialogs.replace(PopupType::Info(t(key))),
        After::ConflictsOr(conflict_key, _) if has_conflicts() => {
            state.dialogs.replace(PopupType::Error(t(conflict_key)))
        }
        After::ConflictsOr(_, ok_key) => state.dialogs.replace(PopupType::Info(t(ok_key))),
    }
}

/// Runs `action` and returns its result, error message key and follow-up.
fn plan(
    repo: &mut Repository,
    action: &GitConfirmedAction,
) -> (anyhow::Result<()>, &'static str, After) {
    use crate::git::{
        branches, cherry_pick, merge, rebase, remote, reset, revert, stage, stash, tags,
    };
    use GitConfirmedAction as A;
    const OK: &str = "git_operation_success";
    match action {
        A::DeleteBranch(name) => (
            branches::delete_branch(repo, name),
            "git_error_delete_branch_failed",
            After::Nothing,
        ),
        A::MergeBranch(name) => (
            merge::merge(repo, name).map(|_| ()),
            "git_error_merge_failed",
            After::ConflictsOr("git_error_merge_conflicts", "git_merge_success"),
        ),
        A::RebaseBranch(onto) => (
            rebase::rebase_branch(repo, onto).map(|_| ()),
            "git_error_rebase_failed",
            After::Info(OK),
        ),
        A::StashDrop(index) => (
            stash::stash_drop(repo, *index),
            "git_error_stash_drop_failed",
            After::Nothing,
        ),
        A::StashPop(index) => (
            stash::stash_pop(repo, *index),
            "git_error_stash_pop_failed",
            After::Nothing,
        ),
        A::StashClear => (
            stash::stash_clear(repo),
            "git_error_stash_clear_failed",
            After::Info(OK),
        ),
        A::ResetCommit(hash, mode) => (
            reset::reset(repo, hash, *mode),
            "git_error_reset_failed",
            After::Nothing,
        ),
        A::DiscardFile(path) => (
            stage::discard_file_changes(repo, path),
            "git_error_discard_failed",
            After::Nothing,
        ),
        A::DeleteRemote(name) => (
            remote::delete_remote(repo, name),
            "git_error_delete_remote_failed",
            After::Nothing,
        ),
        A::AbortMerge => (
            merge::abort_merge(repo),
            "git_error_abort_merge_failed",
            After::Nothing,
        ),
        A::CherryPick(hash) => (
            cherry_pick::cherry_pick(repo, hash).map(|_| ()),
            "git_error_cherry_pick_failed",
            After::ConflictsOr("git_error_cherry_pick_conflicts", OK),
        ),
        A::Revert(hash) => (
            revert::revert(repo, hash).map(|_| ()),
            "git_error_revert_failed",
            After::ConflictsOr("git_error_revert_conflicts", OK),
        ),
        A::DeleteTag(name) => (
            tags::delete_tag(repo, name),
            "git_error_delete_tag_failed",
            After::Info(OK),
        ),
        A::DeleteRemoteBranch { .. } => (Ok(()), "", After::Nothing),
    }
}
