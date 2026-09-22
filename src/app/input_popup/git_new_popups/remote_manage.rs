//! Input handlers for Git Remote Management and Remote Add popups.

use crate::app::context::AppContext;
use crate::app::state::popup::{GitPromptPopup, GitRemoteAddState};
use crate::app::state::types::GitConfirmedAction;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_remote_manage(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitPrompt(GitPromptPopup::RemoteManage(mut manage_state))) =
        state.dialogs.top().cloned()
    {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if manage_state.selected_idx > 0 {
                    manage_state.selected_idx -= 1;
                    state
                        .dialogs
                        .replace(PopupType::GitPrompt(GitPromptPopup::RemoteManage(
                            manage_state,
                        )));
                }
                return Ok(None);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !manage_state.remotes.is_empty()
                    && manage_state.selected_idx + 1 < manage_state.remotes.len()
                {
                    manage_state.selected_idx += 1;
                    state
                        .dialogs
                        .replace(PopupType::GitPrompt(GitPromptPopup::RemoteManage(
                            manage_state,
                        )));
                }
                return Ok(None);
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                let current_popup = state.dialogs.top().cloned().unwrap();
                state
                    .dialogs
                    .replace(PopupType::GitPrompt(GitPromptPopup::RemoteAdd(
                        GitRemoteAddState {
                            repo_path: manage_state.repo_path,
                            name_input: String::new(),
                            url_input: String::new(),
                            focus_url: false,
                            name_cursor: 0,
                            url_cursor: 0,
                            previous_popup: Box::new(current_popup),
                        },
                    )));
                return Ok(None);
            }
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
                if let Some(remote) = manage_state.remotes.get(manage_state.selected_idx) {
                    let current_popup = state.dialogs.top().cloned().unwrap();
                    let msg = crate::config::localization::t("git_confirm_delete_remote")
                        .replace("{}", &remote.name);
                    state
                        .dialogs
                        .replace(PopupType::GitPrompt(GitPromptPopup::ConfirmAction(
                            crate::app::state::popup::GitConfirmActionState {
                                message: msg,
                                repo_path: manage_state.repo_path,
                                action: GitConfirmedAction::DeleteRemote(remote.name.clone()),
                                previous_popup: Box::new(current_popup),
                            },
                        )));
                }
                return Ok(None);
            }
            KeyCode::Esc => {
                state.dialogs.replace(*manage_state.previous_popup);
                return Ok(None);
            }
            _ => {}
        }
        Ok(None)
    } else {
        Err(())
    }
}

pub fn handle_remote_add(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitPrompt(GitPromptPopup::RemoteAdd(mut add_state))) =
        state.dialogs.top().cloned()
    {
        match key.code {
            KeyCode::Tab | KeyCode::BackTab => {
                add_state.focus_url = !add_state.focus_url;
                state
                    .dialogs
                    .replace(PopupType::GitPrompt(GitPromptPopup::RemoteAdd(add_state)));
                return Ok(None);
            }
            KeyCode::Esc => {
                state.dialogs.replace(*add_state.previous_popup);
                return Ok(None);
            }
            KeyCode::Enter => {
                let name = add_state.name_input.trim();
                let url = add_state.url_input.trim();
                if name.is_empty() || url.is_empty() {
                    return Ok(None);
                }
                if let Some(repo) = crate::git::repo::find_repo(&add_state.repo_path) {
                    match crate::git::remote::add_remote(&repo, name, url) {
                        Ok(_) => {
                            if let PopupType::GitPrompt(GitPromptPopup::RemoteManage(
                                mut manage_state,
                            )) = *add_state.previous_popup
                            {
                                manage_state.remotes =
                                    crate::git::remote::list_remotes(&repo).unwrap_or_default();
                                state.dialogs.replace(PopupType::GitPrompt(
                                    GitPromptPopup::RemoteManage(manage_state),
                                ));
                            } else {
                                state.dialogs.replace(*add_state.previous_popup);
                            }
                        }
                        Err(e) => {
                            state.dialogs.replace(PopupType::Error(format!(
                                "{}: {}",
                                t("git_error_add_remote_failed"),
                                e
                            )));
                        }
                    }
                }
                return Ok(None);
            }
            KeyCode::Backspace => {
                if !add_state.focus_url {
                    if !add_state.name_input.is_empty() {
                        add_state.name_input.pop();
                        add_state.name_cursor = add_state.name_input.len();
                    }
                } else if !add_state.url_input.is_empty() {
                    add_state.url_input.pop();
                    add_state.url_cursor = add_state.url_input.len();
                }
                state
                    .dialogs
                    .replace(PopupType::GitPrompt(GitPromptPopup::RemoteAdd(add_state)));
                return Ok(None);
            }
            KeyCode::Char(c) => {
                if !add_state.focus_url {
                    add_state.name_input.push(c);
                    add_state.name_cursor = add_state.name_input.len();
                } else {
                    add_state.url_input.push(c);
                    add_state.url_cursor = add_state.url_input.len();
                }
                state
                    .dialogs
                    .replace(PopupType::GitPrompt(GitPromptPopup::RemoteAdd(add_state)));
                return Ok(None);
            }
            _ => {}
        }
        Ok(None)
    } else {
        Err(())
    }
}
