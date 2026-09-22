//! Key action handlers for each tab in GitPanel (Status, Log, Branches, Stash).

use super::super::refresh::refresh_git_panel;
use crate::app::state::{AppState, GitConfirmedAction, PopupType};
use crate::git::log::CommitInfo;
use crate::git::status::GitFileStatus;
use crossterm::event::KeyCode;
use std::path::Path;

pub fn handle_status_tab(
    state: &mut AppState,
    code: KeyCode,
    repo_path: &Path,
    status_entries: &[GitFileStatus],
    cursor_idx: usize,
) -> bool {
    match code {
        KeyCode::Char(' ') => {
            if let Some(entry) = status_entries.get(cursor_idx)
                && let Some(repo) = crate::git::repo::find_repo(repo_path)
            {
                let res = if entry.is_staged && !entry.is_unstaged {
                    crate::git::stage::unstage_file(&repo, &entry.path)
                } else {
                    crate::git::stage::stage_file(&repo, &entry.path)
                };
                if res.is_ok() {
                    refresh_git_panel(state, repo_path, 0, cursor_idx);
                }
            }
            true
        }
        KeyCode::Char('a') => {
            if let Some(repo) = crate::git::repo::find_repo(repo_path)
                && crate::git::stage::stage_all(&repo).is_ok()
            {
                refresh_git_panel(state, repo_path, 0, cursor_idx);
            }
            true
        }
        KeyCode::Char('A') => {
            if let Some(repo) = crate::git::repo::find_repo(repo_path)
                && crate::git::stage::unstage_all(&repo).is_ok()
            {
                refresh_git_panel(state, repo_path, 0, cursor_idx);
            }
            true
        }
        KeyCode::Char('x') | KeyCode::Delete => {
            if let Some(entry) = status_entries.get(cursor_idx) {
                let current_popup = state.dialogs.top().cloned().unwrap();
                let msg = crate::config::localization::t("git_confirm_discard_file")
                    .replace("{}", &entry.path);
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::ConfirmAction(
                        crate::app::state::popup::GitConfirmActionState {
                            message: msg,
                            repo_path: repo_path.to_path_buf(),
                            action: GitConfirmedAction::DiscardFile(entry.path.clone()),
                            previous_popup: Box::new(current_popup),
                        },
                    ),
                ));
            }
            true
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            if let Some(entry) = status_entries.get(cursor_idx)
                && let Some(repo) = crate::git::repo::find_repo(repo_path)
                && crate::git::repo::add_to_gitignore(&repo, &entry.path).is_ok()
            {
                refresh_git_panel(state, repo_path, 0, cursor_idx);
            }
            true
        }
        KeyCode::Char('X') => {
            if let Some(repo) = crate::git::repo::find_repo(repo_path)
                && repo.state() == git2::RepositoryState::Merge
            {
                let current_popup = state.dialogs.top().cloned().unwrap();
                let msg = crate::config::localization::t("git_confirm_abort_merge");
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::ConfirmAction(
                        crate::app::state::popup::GitConfirmActionState {
                            message: msg,
                            repo_path: repo_path.to_path_buf(),
                            action: GitConfirmedAction::AbortMerge,
                            previous_popup: Box::new(current_popup),
                        },
                    ),
                ));
            }
            true
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            let current_popup = state.dialogs.top().cloned().unwrap();
            state.dialogs.replace(PopupType::GitPrompt(
                crate::app::state::popup::GitPromptPopup::CommitPrompt(
                    crate::app::state::popup::GitCommitPromptState {
                        input: String::new(),
                        cursor_idx: 0,
                        repo_path: repo_path.to_path_buf(),
                        is_amend: false,
                        previous_popup: Some(Box::new(current_popup)),
                    },
                ),
            ));
            true
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Some(entry) = status_entries.get(cursor_idx)
                && let Some(repo) = crate::git::repo::find_repo(repo_path)
            {
                let is_staged = entry.is_staged;
                if let Ok(diff_content) =
                    crate::git::diff::get_file_diff(&repo, &entry.path, is_staged)
                {
                    let current_popup = state.dialogs.top().cloned().unwrap();
                    state.dialogs.replace(PopupType::GitPrompt(
                        crate::app::state::popup::GitPromptPopup::DiffView(
                            crate::app::state::popup::GitDiffViewState {
                                repo_path: repo_path.to_path_buf(),
                                file_path: Some(entry.path.clone()),
                                commit_hash: None,
                                diff_content,
                                scroll_y: 0,
                                previous_popup: Box::new(current_popup),
                            },
                        ),
                    ));
                }
            }
            true
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            let current_popup = state.dialogs.top().cloned().unwrap();
            state.dialogs.replace(PopupType::GitPrompt(
                crate::app::state::popup::GitPromptPopup::StashSavePrompt(
                    crate::app::state::popup::GitStashSavePromptState {
                        input: String::new(),
                        cursor_idx: 0,
                        include_untracked: false,
                        repo_path: repo_path.to_path_buf(),
                        previous_popup: Box::new(current_popup),
                    },
                ),
            ));
            true
        }
        _ => false,
    }
}

pub fn handle_log_tab(
    state: &mut AppState,
    code: KeyCode,
    repo_path: &Path,
    log_entries: &[CommitInfo],
    cursor_idx: usize,
) -> bool {
    match code {
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Some(commit) = log_entries.get(cursor_idx)
                && let Some(repo) = crate::git::repo::find_repo(repo_path)
                && let Ok(diff_content) =
                    crate::git::diff::get_commit_diff(&repo, &commit.hash_full)
            {
                let current_popup = state.dialogs.top().cloned().unwrap();
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::DiffView(
                        crate::app::state::popup::GitDiffViewState {
                            repo_path: repo_path.to_path_buf(),
                            file_path: None,
                            commit_hash: Some(commit.hash_short.clone()),
                            diff_content,
                            scroll_y: 0,
                            previous_popup: Box::new(current_popup),
                        },
                    ),
                ));
            }
            true
        }
        KeyCode::Char('s') => {
            if let Some(commit) = log_entries.get(cursor_idx) {
                let current_popup = state.dialogs.top().cloned().unwrap();
                let mode_str = crate::config::localization::t("git_reset_mode_soft");
                let msg = crate::config::localization::t("git_confirm_reset")
                    .replace("{commit}", &commit.hash_short)
                    .replace("{mode}", &mode_str);
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::ConfirmAction(
                        crate::app::state::popup::GitConfirmActionState {
                            message: msg,
                            repo_path: repo_path.to_path_buf(),
                            action: GitConfirmedAction::ResetCommit(
                                commit.hash_full.clone(),
                                crate::git::reset::ResetMode::Soft,
                            ),
                            previous_popup: Box::new(current_popup),
                        },
                    ),
                ));
            }
            true
        }
        KeyCode::Char('x') => {
            if let Some(commit) = log_entries.get(cursor_idx) {
                let current_popup = state.dialogs.top().cloned().unwrap();
                let mode_str = crate::config::localization::t("git_reset_mode_mixed");
                let msg = crate::config::localization::t("git_confirm_reset")
                    .replace("{commit}", &commit.hash_short)
                    .replace("{mode}", &mode_str);
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::ConfirmAction(
                        crate::app::state::popup::GitConfirmActionState {
                            message: msg,
                            repo_path: repo_path.to_path_buf(),
                            action: GitConfirmedAction::ResetCommit(
                                commit.hash_full.clone(),
                                crate::git::reset::ResetMode::Mixed,
                            ),
                            previous_popup: Box::new(current_popup),
                        },
                    ),
                ));
            }
            true
        }
        KeyCode::Char('h') => {
            if let Some(commit) = log_entries.get(cursor_idx) {
                let current_popup = state.dialogs.top().cloned().unwrap();
                let mode_str = crate::config::localization::t("git_reset_mode_hard");
                let msg = crate::config::localization::t("git_confirm_reset")
                    .replace("{commit}", &commit.hash_short)
                    .replace("{mode}", &mode_str);
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::ConfirmAction(
                        crate::app::state::popup::GitConfirmActionState {
                            message: msg,
                            repo_path: repo_path.to_path_buf(),
                            action: GitConfirmedAction::ResetCommit(
                                commit.hash_full.clone(),
                                crate::git::reset::ResetMode::Hard,
                            ),
                            previous_popup: Box::new(current_popup),
                        },
                    ),
                ));
            }
            true
        }
        KeyCode::Char('b') | KeyCode::Char('B') | KeyCode::Char('n') | KeyCode::Char('N') => {
            if let Some(commit) = log_entries.get(cursor_idx) {
                let current_popup = state.dialogs.top().cloned().unwrap();
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::BranchCreatePrompt(
                        crate::app::state::popup::GitBranchCreatePromptState {
                            input: String::new(),
                            cursor_idx: 0,
                            start_point: commit.hash_full.clone(),
                            repo_path: repo_path.to_path_buf(),
                            previous_popup: Box::new(current_popup),
                        },
                    ),
                ));
            }
            true
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            if let Some(commit) = log_entries.get(cursor_idx) {
                let current_popup = state.dialogs.top().cloned().unwrap();
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::TagCreatePrompt(
                        crate::app::state::popup::GitTagCreatePromptState {
                            input: String::new(),
                            cursor_idx: 0,
                            target: commit.hash_full.clone(),
                            repo_path: repo_path.to_path_buf(),
                            previous_popup: Box::new(current_popup),
                        },
                    ),
                ));
            }
            true
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if let Some(commit) = log_entries.get(cursor_idx) {
                let current_popup = state.dialogs.top().cloned().unwrap();
                let msg = crate::config::localization::t("git_confirm_cherry_pick")
                    .replace("{}", &commit.hash_short);
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::ConfirmAction(
                        crate::app::state::popup::GitConfirmActionState {
                            message: msg,
                            repo_path: repo_path.to_path_buf(),
                            action: GitConfirmedAction::CherryPick(commit.hash_full.clone()),
                            previous_popup: Box::new(current_popup),
                        },
                    ),
                ));
            }
            true
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if let Some(commit) = log_entries.get(cursor_idx) {
                let current_popup = state.dialogs.top().cloned().unwrap();
                let msg = crate::config::localization::t("git_confirm_revert")
                    .replace("{}", &commit.hash_short);
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::ConfirmAction(
                        crate::app::state::popup::GitConfirmActionState {
                            message: msg,
                            repo_path: repo_path.to_path_buf(),
                            action: GitConfirmedAction::Revert(commit.hash_full.clone()),
                            previous_popup: Box::new(current_popup),
                        },
                    ),
                ));
            }
            true
        }
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(commit) = log_entries.get(cursor_idx) {
                let text = commit.hash_full.clone();
                match crate::app::sys_helpers::clipboard::set_text(&text) {
                    Ok(()) => {
                        let msg = crate::config::localization::t("git_hash_copied")
                            .replace("{}", &commit.hash_short);
                        state.dialogs.push(PopupType::Info(msg));
                    }
                    Err(e) => {
                        let msg = crate::config::localization::t("clipboard_failed")
                            .replace("{}", &e.to_string());
                        state.dialogs.push(PopupType::Error(msg));
                    }
                }
            }
            true
        }
        KeyCode::Enter => {
            if let Some(commit) = log_entries.get(cursor_idx) {
                let current_popup = state.dialogs.top().cloned();
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::ConfirmCheckout(
                        crate::app::state::popup::GitConfirmCheckoutState {
                            target: commit.hash_full.clone(),
                            is_branch: false,
                            repo_path: repo_path.to_path_buf(),
                            previous_popup: current_popup.map(Box::new),
                        },
                    ),
                ));
            }
            true
        }
        _ => false,
    }
}
