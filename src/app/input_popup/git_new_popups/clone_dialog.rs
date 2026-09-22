use crate::app::context::AppContext;
use crate::app::state::popup::GitPromptPopup;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};

use crate::keybindings::actions::Action;

pub fn handle_clone(
    state: &mut AppState,
    key: KeyEvent,
    context: &AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitPrompt(GitPromptPopup::ClonePrompt(mut clone_state))) =
        state.dialogs.take()
    {
        match key.code {
            KeyCode::Tab | KeyCode::BackTab | KeyCode::Down | KeyCode::Up => {
                clone_state.focus_dir = !clone_state.focus_dir;
                state
                    .dialogs
                    .replace(PopupType::GitPrompt(GitPromptPopup::ClonePrompt(
                        clone_state,
                    )));
                Ok(None)
            }
            KeyCode::Esc => {
                // Dismiss clone dialog
                Ok(None)
            }
            KeyCode::Enter => {
                let url = clone_state.url_input.trim();
                if url.is_empty() {
                    state
                        .dialogs
                        .replace(PopupType::GitPrompt(GitPromptPopup::ClonePrompt(
                            clone_state,
                        )));
                    return Ok(None);
                }

                let dir_name = if !clone_state.dir_input.trim().is_empty() {
                    clone_state.dir_input.trim().to_string()
                } else {
                    let clean = url.trim_end_matches('/').trim_end_matches(".git");
                    let last = clean.split(['/', ':', '\\']).next_back().unwrap_or("repo");
                    if last.is_empty() {
                        "repo".to_string()
                    } else {
                        last.to_string()
                    }
                };

                let target_path = clone_state.target_parent_path.join(&dir_name);
                if target_path.exists() {
                    state
                        .dialogs
                        .replace(PopupType::Error(t("git_clone_dir_exists")));
                    return Ok(None);
                }

                match crate::git::repo::clone_repo(url, &target_path) {
                    Ok(_) => {
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        state
                            .dialogs
                            .replace(PopupType::Info(t("git_clone_success")));
                    }
                    Err(e) => {
                        state.dialogs.replace(PopupType::Error(format!(
                            "{}: {}",
                            t("git_clone_error"),
                            e
                        )));
                    }
                }
                Ok(None)
            }
            KeyCode::Backspace => {
                if !clone_state.focus_dir {
                    if !clone_state.url_input.is_empty() {
                        clone_state.url_input.pop();
                        clone_state.url_cursor = clone_state.url_input.len();
                    }
                } else if !clone_state.dir_input.is_empty() {
                    clone_state.dir_input.pop();
                    clone_state.dir_cursor = clone_state.dir_input.len();
                }
                state
                    .dialogs
                    .replace(PopupType::GitPrompt(GitPromptPopup::ClonePrompt(
                        clone_state,
                    )));
                Ok(None)
            }
            KeyCode::Char(c) => {
                if !clone_state.focus_dir {
                    clone_state.url_input.push(c);
                    clone_state.url_cursor = clone_state.url_input.len();
                } else {
                    clone_state.dir_input.push(c);
                    clone_state.dir_cursor = clone_state.dir_input.len();
                }
                state
                    .dialogs
                    .replace(PopupType::GitPrompt(GitPromptPopup::ClonePrompt(
                        clone_state,
                    )));
                Ok(None)
            }
            _ => {
                state
                    .dialogs
                    .replace(PopupType::GitPrompt(GitPromptPopup::ClonePrompt(
                        clone_state,
                    )));
                Ok(None)
            }
        }
    } else {
        Err(())
    }
}
