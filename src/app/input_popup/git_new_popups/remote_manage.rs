//! Input handlers for Git Remote Management and Remote Add popups.

use crate::app::context::AppContext;
use crate::app::form::FieldKey;
use crate::app::state::popup::{GitConfirmActionState, GitPromptPopup, GitRemoteAddState};
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
    let Some(PopupType::GitPrompt(GitPromptPopup::RemoteManage(manage))) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            manage.selected_idx = manage.selected_idx.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if manage.selected_idx + 1 < manage.remotes.len() {
                manage.selected_idx += 1;
            }
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            let repo_path = manage.repo_path.clone();
            state.dialogs.open_over(|previous_popup| {
                PopupType::GitPrompt(GitPromptPopup::RemoteAdd(GitRemoteAddState {
                    repo_path,
                    fields: Default::default(),
                    previous_popup,
                }))
            });
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
            if let Some(remote) = manage.remotes.get(manage.selected_idx) {
                let message = t("git_confirm_delete_remote").replace("{}", &remote.name);
                let action = GitConfirmedAction::DeleteRemote(remote.name.clone());
                let repo_path = manage.repo_path.clone();
                state.dialogs.open_over(|previous_popup| {
                    PopupType::GitPrompt(GitPromptPopup::ConfirmAction(GitConfirmActionState {
                        message,
                        repo_path,
                        action,
                        previous_popup,
                    }))
                });
            }
        }
        KeyCode::Esc => {
            if let Some(PopupType::GitPrompt(GitPromptPopup::RemoteManage(manage))) =
                state.dialogs.pop()
            {
                state.dialogs.replace(*manage.previous_popup);
            }
        }
        _ => {}
    }
    Ok(None)
}

pub fn handle_remote_add(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::GitPrompt(GitPromptPopup::RemoteAdd(add))) = state.dialogs.top_mut() else {
        return Err(());
    };
    match add.fields.handle_key(&key) {
        FieldKey::Cancel => {
            if let Some(PopupType::GitPrompt(GitPromptPopup::RemoteAdd(add))) = state.dialogs.pop()
            {
                state.dialogs.replace(*add.previous_popup);
            }
        }
        FieldKey::Submit => {
            if add.fields.first().trim().is_empty() || add.fields.second().trim().is_empty() {
                return Ok(None);
            }
            if let Some(PopupType::GitPrompt(GitPromptPopup::RemoteAdd(add))) = state.dialogs.pop()
            {
                add_remote(state, add);
            }
        }
        FieldKey::Handled | FieldKey::Other => {}
    }
    Ok(None)
}

/// Adds the remote and returns to the (refreshed) remote list.
fn add_remote(state: &mut AppState, add: GitRemoteAddState) {
    let Some(repo) = crate::git::repo::find_repo(&add.repo_path) else {
        // Keep the dialog open, as before, when the repository is gone.
        state
            .dialogs
            .push(PopupType::GitPrompt(GitPromptPopup::RemoteAdd(add)));
        return;
    };
    let name = add.fields.first().trim();
    let url = add.fields.second().trim();
    match crate::git::remote::add_remote(&repo, name, url) {
        Ok(_) => match *add.previous_popup {
            PopupType::GitPrompt(GitPromptPopup::RemoteManage(mut manage)) => {
                manage.remotes = crate::git::remote::list_remotes(&repo).unwrap_or_default();
                state
                    .dialogs
                    .replace(PopupType::GitPrompt(GitPromptPopup::RemoteManage(manage)));
            }
            previous => state.dialogs.replace(previous),
        },
        Err(e) => state.dialogs.replace(PopupType::Error(format!(
            "{}: {}",
            t("git_error_add_remote_failed"),
            e
        ))),
    }
}
