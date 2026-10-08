//! Key action handlers for the Branches and Stash tabs of the Git panel.
//!
//! Repository work runs in the background (`app::git_local`); the handlers
//! only pick the entry under the cursor and open dialogs.

use crate::app::git_local::GitContext;
use crate::app::input_popup::git_new_popups::{open_confirm, open_diff, open_name_prompt};
use crate::app::state::popup::{
    GitConfirmCheckoutState, GitNameAction, GitPromptPopup, GitRemoteManageState,
};
use crate::app::state::{AppState, GitConfirmedAction, PopupType};
use crate::config::localization::t;
use crate::git::branches::BranchInfo;
use crate::git::stash::StashInfo;
use crossterm::event::KeyCode;
use std::path::Path;

pub fn handle_branch_tab(
    state: &mut AppState,
    code: KeyCode,
    repo_path: &Path,
    branch_entries: &[BranchInfo],
    cursor_idx: usize,
    current_branch: &str,
) -> bool {
    let branch = branch_entries.get(cursor_idx);
    match code {
        KeyCode::Char('n' | 'N') => open_name_prompt(
            state,
            repo_path,
            GitNameAction::CreateBranch {
                start_point: "HEAD".to_string(),
            },
        ),
        KeyCode::Char('d' | 'D') | KeyCode::Delete => {
            if let Some(branch) = branch {
                confirm_delete_branch(state, repo_path, branch);
            }
        }
        KeyCode::Char('r') => {
            if let Some(branch) = branch.filter(|b| !b.is_remote) {
                let old_name = branch.name.clone();
                open_name_prompt(state, repo_path, GitNameAction::RenameBranch { old_name });
            }
        }
        KeyCode::Char('R') => open_remote_manager(state, repo_path),
        KeyCode::Char('m' | 'M') => {
            if let Some(branch) = branch.filter(|b| !b.is_current) {
                let msg = t("git_confirm_merge_branch")
                    .replace("{source}", &branch.name)
                    .replace("{target}", current_branch);
                let action = GitConfirmedAction::MergeBranch(branch.name.clone());
                open_confirm(state, repo_path, msg, action);
            }
        }
        KeyCode::Char('b' | 'B') => {
            if let Some(branch) = branch.filter(|b| !b.is_current) {
                let msg = t("git_confirm_rebase")
                    .replace("{current}", current_branch)
                    .replace("{onto}", &branch.name);
                let action = GitConfirmedAction::RebaseBranch(branch.name.clone());
                open_confirm(state, repo_path, msg, action);
            }
        }
        KeyCode::Enter => {
            if let Some(branch) = branch {
                open_checkout(state, repo_path, branch.name.clone(), true);
            }
        }
        _ => return false,
    }
    true
}

/// Opens the checkout confirmation of `target` over the Git panel.
pub fn open_checkout(state: &mut AppState, repo_path: &Path, target: String, is_branch: bool) {
    state.dialogs.open_over(|current_popup| {
        PopupType::GitPrompt(GitPromptPopup::ConfirmCheckout(GitConfirmCheckoutState {
            target,
            is_branch,
            repo_path: repo_path.to_path_buf(),
            previous_popup: Some(current_popup),
        }))
    });
}

/// Asks to delete a local branch, or a remote one (`remote/branch`); the
/// checked-out branch cannot be deleted.
fn confirm_delete_branch(state: &mut AppState, repo_path: &Path, branch: &BranchInfo) {
    if branch.is_current {
        state.dialogs.push(PopupType::Error(t(
            "git_error_cannot_delete_current_branch",
        )));
        return;
    }
    let (msg, action) = if branch.is_remote {
        let Some((remote, name)) = branch.name.split_once('/') else {
            return;
        };
        let msg = t("git_confirm_delete_remote_branch")
            .replace("{remote}", remote)
            .replace("{branch}", name);
        let action = GitConfirmedAction::DeleteRemoteBranch {
            remote: remote.to_string(),
            branch: name.to_string(),
        };
        (msg, action)
    } else {
        let msg = t("git_confirm_delete_branch").replace("{}", &branch.name);
        (msg, GitConfirmedAction::DeleteBranch(branch.name.clone()))
    };
    open_confirm(state, repo_path, msg, action);
}

/// Lists the remotes in the background and opens the remote manager.
fn open_remote_manager(state: &mut AppState, repo_path: &Path) {
    let repo_path_buf = repo_path.to_path_buf();
    state.run_git_local(
        repo_path,
        |repo| Ok(crate::git::remote::list_remotes(repo).unwrap_or_default()),
        move |state, remotes| {
            state.dialogs.open_over(|previous_popup| {
                PopupType::GitPrompt(GitPromptPopup::RemoteManage(GitRemoteManageState {
                    repo_path: repo_path_buf,
                    remotes,
                    selected_idx: 0,
                    previous_popup,
                }))
            });
        },
    );
}

/// Asks to run `action` on the stash at `index` (message key with `{}`).
fn confirm_on_stash(
    state: &mut AppState,
    repo_path: &Path,
    index: usize,
    key: &str,
    action: GitConfirmedAction,
) {
    let msg = t(key).replace("{}", &index.to_string());
    open_confirm(state, repo_path, msg, action);
}

/// Applies the stash at `index` in the background, then reports success
/// over the refreshed panel.
fn apply_stash(state: &mut AppState, repo_path: &Path, index: usize) {
    let reload = repo_path.to_path_buf();
    state.run_git_local(
        repo_path,
        move |repo| crate::git::stash::stash_apply(repo, index).ctx("git_error_stash_apply_failed"),
        move |state, ()| {
            state.reload_git_panel(&reload);
            state
                .dialogs
                .push(PopupType::Info(t("git_operation_success")));
        },
    );
}

pub fn handle_stash_tab(
    state: &mut AppState,
    code: KeyCode,
    repo_path: &Path,
    stash_entries: &[StashInfo],
    cursor_idx: usize,
) -> bool {
    let stash = stash_entries.get(cursor_idx);
    match (code, stash) {
        (KeyCode::Char('a' | 'A'), Some(stash)) => apply_stash(state, repo_path, stash.index),
        (KeyCode::Char('p' | 'P') | KeyCode::Enter, Some(stash)) => confirm_on_stash(
            state,
            repo_path,
            stash.index,
            "git_confirm_stash_pop",
            GitConfirmedAction::StashPop(stash.index),
        ),
        (KeyCode::Char('d' | 'D'), Some(stash)) => {
            let (oid, label) = (stash.oid.clone(), format!("stash@{{{}}}", stash.index));
            open_diff(state, repo_path, None, Some(label), move |r| {
                crate::git::diff::get_stash_diff(r, &oid)
            });
        }
        (KeyCode::Delete | KeyCode::Char('x' | 'X'), Some(stash)) => confirm_on_stash(
            state,
            repo_path,
            stash.index,
            "git_confirm_stash_drop",
            GitConfirmedAction::StashDrop(stash.index),
        ),
        (KeyCode::Char('C'), Some(_)) => open_confirm(
            state,
            repo_path,
            t("git_confirm_stash_clear"),
            GitConfirmedAction::StashClear,
        ),
        (
            KeyCode::Char('a' | 'A' | 'p' | 'P' | 'd' | 'D' | 'x' | 'X' | 'C')
            | KeyCode::Enter
            | KeyCode::Delete,
            None,
        ) => {}
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_prompt_starts_with_the_name_field_focused() {
        let mut state = AppState::new(".".into(), ".".into());
        state.dialogs.replace(PopupType::Info("panel".into()));
        let branches = [BranchInfo {
            name: "feature".into(),
            is_current: false,
            is_remote: false,
            ahead: 0,
            behind: 0,
        }];
        assert!(handle_branch_tab(
            &mut state,
            KeyCode::Char('r'),
            Path::new("."),
            &branches,
            0,
            "main",
        ));
        match state.dialogs.top() {
            Some(PopupType::GitPrompt(GitPromptPopup::NamePrompt(p))) => {
                assert_eq!(p.cursor_idx, 0);
                assert_eq!(p.input, "feature");
                assert!(matches!(p.action, GitNameAction::RenameBranch { .. }));
            }
            other => panic!("expected rename prompt, got {other:?}"),
        }
    }
}
