//! Key action handlers for each tab in GitPanel (Status, Log, Branches, Stash).

use super::super::refresh::refresh_git_panel;
use crate::app::state::{AppState, GitConfirmedAction, PopupType};
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
    match code {
        KeyCode::Char('n') | KeyCode::Char('N') => {
            state.dialogs.open_over(|current_popup| {
                PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::BranchCreatePrompt(
                        crate::app::state::popup::GitBranchCreatePromptState {
                            input: String::new(),
                            cursor_idx: 0,
                            start_point: "HEAD".to_string(),
                            repo_path: repo_path.to_path_buf(),
                            previous_popup: current_popup,
                        },
                    ),
                )
            });
            true
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
            if let Some(branch) = branch_entries.get(cursor_idx) {
                if branch.is_current {
                    state
                        .dialogs
                        .push(PopupType::Error(crate::config::localization::t(
                            "git_error_cannot_delete_current_branch",
                        )));
                    return true;
                }
                if branch.is_remote {
                    if let Some((remote, b_name)) = branch.name.split_once('/') {
                        let msg =
                            crate::config::localization::t("git_confirm_delete_remote_branch")
                                .replace("{remote}", remote)
                                .replace("{branch}", b_name);
                        state.dialogs.open_over(|current_popup| {
                            PopupType::GitPrompt(
                                crate::app::state::popup::GitPromptPopup::ConfirmAction(
                                    crate::app::state::popup::GitConfirmActionState {
                                        message: msg,
                                        repo_path: repo_path.to_path_buf(),
                                        action: GitConfirmedAction::DeleteRemoteBranch {
                                            remote: remote.to_string(),
                                            branch: b_name.to_string(),
                                        },
                                        previous_popup: current_popup,
                                    },
                                ),
                            )
                        });
                    }
                } else {
                    let msg = crate::config::localization::t("git_confirm_delete_branch")
                        .replace("{}", &branch.name);
                    state.dialogs.open_over(|current_popup| {
                        PopupType::GitPrompt(
                            crate::app::state::popup::GitPromptPopup::ConfirmAction(
                                crate::app::state::popup::GitConfirmActionState {
                                    message: msg,
                                    repo_path: repo_path.to_path_buf(),
                                    action: GitConfirmedAction::DeleteBranch(branch.name.clone()),
                                    previous_popup: current_popup,
                                },
                            ),
                        )
                    });
                }
            }
            true
        }
        KeyCode::Char('r') => {
            if let Some(branch) = branch_entries.get(cursor_idx)
                && !branch.is_remote
            {
                state.dialogs.open_over(|current_popup| {
                    PopupType::GitPrompt(
                        crate::app::state::popup::GitPromptPopup::BranchRenamePrompt(
                            crate::app::state::popup::GitBranchRenamePromptState {
                                input: branch.name.clone(),
                                cursor_idx: 0,
                                old_name: branch.name.clone(),
                                repo_path: repo_path.to_path_buf(),
                                previous_popup: current_popup,
                            },
                        ),
                    )
                });
            }
            true
        }
        KeyCode::Char('R') => {
            if let Some(repo) = crate::git::repo::find_repo(repo_path) {
                let remotes = crate::git::remote::list_remotes(&repo).unwrap_or_default();
                state.dialogs.open_over(|current_popup| {
                    PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::RemoteManage(
                        crate::app::state::popup::GitRemoteManageState {
                            repo_path: repo_path.to_path_buf(),
                            remotes,
                            selected_idx: 0,
                            previous_popup: current_popup,
                        },
                    ))
                });
            }
            true
        }
        KeyCode::Char('m') | KeyCode::Char('M') => {
            if let Some(branch) = branch_entries.get(cursor_idx)
                && !branch.is_current
            {
                let msg = crate::config::localization::t("git_confirm_merge_branch")
                    .replace("{source}", &branch.name)
                    .replace("{target}", current_branch);
                state.dialogs.open_over(|current_popup| {
                    PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::ConfirmAction(
                        crate::app::state::popup::GitConfirmActionState {
                            message: msg,
                            repo_path: repo_path.to_path_buf(),
                            action: GitConfirmedAction::MergeBranch(branch.name.clone()),
                            previous_popup: current_popup,
                        },
                    ))
                });
            }
            true
        }
        KeyCode::Char('b') | KeyCode::Char('B') => {
            if let Some(branch) = branch_entries.get(cursor_idx)
                && !branch.is_current
            {
                let msg = crate::config::localization::t("git_confirm_rebase")
                    .replace("{current}", current_branch)
                    .replace("{onto}", &branch.name);
                state.dialogs.open_over(|current_popup| {
                    PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::ConfirmAction(
                        crate::app::state::popup::GitConfirmActionState {
                            message: msg,
                            repo_path: repo_path.to_path_buf(),
                            action: GitConfirmedAction::RebaseBranch(branch.name.clone()),
                            previous_popup: current_popup,
                        },
                    ))
                });
            }
            true
        }
        KeyCode::Enter => {
            if let Some(branch) = branch_entries.get(cursor_idx) {
                state.dialogs.open_over(|current_popup| {
                    PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::ConfirmCheckout(
                        crate::app::state::popup::GitConfirmCheckoutState {
                            target: branch.name.clone(),
                            is_branch: true,
                            repo_path: repo_path.to_path_buf(),
                            previous_popup: Some(current_popup),
                        },
                    ))
                });
            }
            true
        }
        _ => false,
    }
}

pub fn handle_stash_tab(
    state: &mut AppState,
    code: KeyCode,
    repo_path: &Path,
    stash_entries: &[StashInfo],
    cursor_idx: usize,
) -> bool {
    match code {
        KeyCode::Char('a') | KeyCode::Char('A') => {
            if let Some(stash) = stash_entries.get(cursor_idx)
                && let Some(mut repo) = crate::git::repo::find_repo(repo_path)
                && crate::git::stash::stash_apply(&mut repo, stash.index).is_ok()
            {
                refresh_git_panel(state, repo_path, 3, cursor_idx);
                state
                    .dialogs
                    .replace(PopupType::Info(crate::config::localization::t(
                        "git_operation_success",
                    )));
            }
            true
        }
        KeyCode::Char('p') | KeyCode::Char('P') | KeyCode::Enter => {
            if let Some(stash) = stash_entries.get(cursor_idx) {
                let msg = crate::config::localization::t("git_confirm_stash_pop")
                    .replace("{}", &stash.index.to_string());
                state.dialogs.open_over(|current_popup| {
                    PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::ConfirmAction(
                        crate::app::state::popup::GitConfirmActionState {
                            message: msg,
                            repo_path: repo_path.to_path_buf(),
                            action: GitConfirmedAction::StashPop(stash.index),
                            previous_popup: current_popup,
                        },
                    ))
                });
            }
            true
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Some(stash) = stash_entries.get(cursor_idx)
                && let Some(repo) = crate::git::repo::find_repo(repo_path)
                && let Ok(diff_content) = crate::git::diff::get_stash_diff(&repo, &stash.oid)
            {
                state.dialogs.open_over(|current_popup| {
                    PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::DiffView(
                        crate::app::state::popup::GitDiffViewState {
                            repo_path: repo_path.to_path_buf(),
                            file_path: None,
                            commit_hash: Some(format!("stash@{{{}}}", stash.index)),
                            diff_content,
                            scroll_y: 0,
                            previous_popup: current_popup,
                        },
                    ))
                });
            }
            true
        }
        KeyCode::Delete | KeyCode::Char('x') | KeyCode::Char('X') => {
            if let Some(stash) = stash_entries.get(cursor_idx) {
                let msg = crate::config::localization::t("git_confirm_stash_drop")
                    .replace("{}", &stash.index.to_string());
                state.dialogs.open_over(|current_popup| {
                    PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::ConfirmAction(
                        crate::app::state::popup::GitConfirmActionState {
                            message: msg,
                            repo_path: repo_path.to_path_buf(),
                            action: GitConfirmedAction::StashDrop(stash.index),
                            previous_popup: current_popup,
                        },
                    ))
                });
            }
            true
        }
        KeyCode::Char('C') => {
            if !stash_entries.is_empty() {
                let msg = crate::config::localization::t("git_confirm_stash_clear");
                state.dialogs.open_over(|current_popup| {
                    PopupType::GitPrompt(crate::app::state::popup::GitPromptPopup::ConfirmAction(
                        crate::app::state::popup::GitConfirmActionState {
                            message: msg,
                            repo_path: repo_path.to_path_buf(),
                            action: GitConfirmedAction::StashClear,
                            previous_popup: current_popup,
                        },
                    ))
                });
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::popup::GitPromptPopup;

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
            Some(PopupType::GitPrompt(GitPromptPopup::BranchRenamePrompt(p))) => {
                assert_eq!(p.cursor_idx, 0);
                assert_eq!(p.input, "feature");
            }
            other => panic!("expected rename prompt, got {other:?}"),
        }
    }
}
