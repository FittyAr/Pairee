//! Key action handlers for the Tags tab in GitPanel.

use crate::app::state::{AppState, GitConfirmedAction, PopupType};
use crate::git::tags::TagInfo;
use crossterm::event::KeyCode;
use std::path::Path;

pub fn handle_tag_tab(
    state: &mut AppState,
    code: KeyCode,
    repo_path: &Path,
    tag_entries: &[TagInfo],
    cursor_idx: usize,
) -> bool {
    match code {
        KeyCode::Char('n') | KeyCode::Char('N') => {
            let current_popup = state.dialogs.top().cloned().unwrap();
            state.dialogs.replace(PopupType::GitPrompt(
                crate::app::state::popup::GitPromptPopup::TagCreatePrompt(
                    crate::app::state::popup::GitTagCreatePromptState {
                        input: String::new(),
                        cursor_idx: 0,
                        target: "HEAD".to_string(),
                        repo_path: repo_path.to_path_buf(),
                        previous_popup: Box::new(current_popup),
                    },
                ),
            ));
            true
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
            if let Some(tag) = tag_entries.get(cursor_idx) {
                let current_popup = state.dialogs.top().cloned().unwrap();
                let msg = crate::config::localization::t("git_confirm_delete_tag")
                    .replace("{}", &tag.name);
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::ConfirmAction(
                        crate::app::state::popup::GitConfirmActionState {
                            message: msg,
                            repo_path: repo_path.to_path_buf(),
                            action: GitConfirmedAction::DeleteTag(tag.name.clone()),
                            previous_popup: Box::new(current_popup),
                        },
                    ),
                ));
            }
            true
        }
        KeyCode::Enter => {
            if let Some(tag) = tag_entries.get(cursor_idx) {
                let current_popup = state.dialogs.top().cloned();
                state.dialogs.replace(PopupType::GitPrompt(
                    crate::app::state::popup::GitPromptPopup::ConfirmCheckout(
                        crate::app::state::popup::GitConfirmCheckoutState {
                            target: tag.name.clone(),
                            is_branch: false,
                            repo_path: repo_path.to_path_buf(),
                            previous_popup: current_popup.map(Box::new),
                        },
                    ),
                ));
            }
            true
        }
        KeyCode::Char('u') | KeyCode::Char('U') => {
            if let Some(repo) = crate::git::repo::find_repo(repo_path) {
                let remote = crate::git::remote::resolve_remote_name(&repo, None)
                    .unwrap_or_else(|_| "origin".to_string());
                match crate::git::tags::push_tags(&repo, &remote) {
                    Ok(_) => {
                        state
                            .dialogs
                            .replace(PopupType::Info(crate::config::localization::t(
                                "git_operation_success",
                            )));
                    }
                    Err(e) => {
                        state.dialogs.replace(PopupType::Error(format!(
                            "{}: {}",
                            crate::config::localization::t("git_error_push_tags_failed"),
                            e
                        )));
                    }
                }
            }
            true
        }
        _ => false,
    }
}
