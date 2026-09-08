//! Git remote operations (fetch, pull, push) for git_panel.

use super::refresh::refresh_git_panel;
use crate::app::state::{AppState, PopupType};
use std::path::Path;

pub fn handle_fetch(state: &mut AppState, repo_path: &Path) {
    if let Some(repo) = crate::git::repo::find_repo(repo_path) {
        match crate::git::remote::fetch(&repo, "origin") {
            Ok(_) => {
                state
                    .dialogs
                    .replace(PopupType::Info(crate::config::localization::t(
                        "git_operation_success",
                    )));
            }
            Err(e) => {
                state
                    .dialogs
                    .replace(PopupType::Error(format!("Fetch failed: {}", e)));
            }
        }
    }
}

pub fn handle_pull(state: &mut AppState, repo_path: &Path, active_tab: usize, cursor_idx: usize) {
    if let Some(repo) = crate::git::repo::find_repo(repo_path) {
        let current_branch_name = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().ok().map(|s| s.to_string()))
            .unwrap_or_else(|| "main".to_string());
        match crate::git::remote::pull(&repo, "origin", &current_branch_name) {
            Ok(_) => {
                refresh_git_panel(state, repo_path, active_tab, cursor_idx);
                state
                    .dialogs
                    .replace(PopupType::Info(crate::config::localization::t(
                        "git_operation_success",
                    )));
            }
            Err(e) => {
                state
                    .dialogs
                    .replace(PopupType::Error(format!("Pull failed: {}", e)));
            }
        }
    }
}

pub fn handle_push(state: &mut AppState, repo_path: &Path) {
    if let Some(repo) = crate::git::repo::find_repo(repo_path) {
        let current_branch_name = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().ok().map(|s| s.to_string()))
            .unwrap_or_else(|| "main".to_string());
        match crate::git::remote::push(&repo, "origin", &current_branch_name) {
            Ok(_) => {
                state
                    .dialogs
                    .replace(PopupType::Info(crate::config::localization::t(
                        "git_operation_success",
                    )));
            }
            Err(e) => {
                state
                    .dialogs
                    .replace(PopupType::Error(format!("Push failed: {}", e)));
            }
        }
    }
}
