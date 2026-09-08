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
                let res = match entry.kind {
                    crate::git::status::StatusKind::Added => {
                        crate::git::stage::unstage_file(&repo, &entry.path)
                    }
                    _ => crate::git::stage::stage_file(&repo, &entry.path),
                };
                if res.is_ok() {
                    refresh_git_panel(state, repo_path, 0, cursor_idx);
                }
            }
            true
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            state.dialogs.replace(PopupType::GitPrompt(
                crate::app::state::popup::GitPromptPopup::CommitPrompt(
                    crate::app::state::popup::GitCommitPromptState {
                        input: String::new(),
                        cursor_idx: 0,
                        repo_path: repo_path.to_path_buf(),
                    },
                ),
            ));
            true
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Some(entry) = status_entries.get(cursor_idx)
                && let Some(repo) = crate::git::repo::find_repo(repo_path)
            {
                let is_staged = matches!(entry.kind, crate::git::status::StatusKind::Added);
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
                let msg = crate::config::localization::t("git_confirm_reset")
                    .replace("{}", &commit.hash_short)
                    .replace("{}", "Soft");
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
                let msg = crate::config::localization::t("git_confirm_reset")
                    .replace("{}", &commit.hash_short)
                    .replace("{}", "Mixed");
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
                let msg = crate::config::localization::t("git_confirm_reset")
                    .replace("{}", &commit.hash_short)
                    .replace("{}", "Hard");
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
        KeyCode::Enter => {
            if let Some(commit) = log_entries.get(cursor_idx) {
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::ConfirmCheckout(
                        crate::app::state::popup::GitConfirmCheckoutState {
                            target: commit.hash_full.clone(),
                            is_branch: false,
                            repo_path: repo_path.to_path_buf(),
                        },
                    ),
                ));
            }
            true
        }
        _ => false,
    }
}
