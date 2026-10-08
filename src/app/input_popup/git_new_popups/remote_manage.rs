//! Input handlers for Git Remote Management and Remote Add popups.

use super::confirm::open_confirm;
use crate::app::context::AppContext;
use crate::app::form::FieldKey;
use crate::app::git_local::{GitContext, GitFailure};
use crate::app::state::popup::{GitPromptPopup, GitRemoteAddState};
use crate::app::state::types::GitConfirmedAction;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::git::remote::RemoteInfo;
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
                open_confirm(state, &repo_path, message, action);
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

/// Adds the remote in the background and returns to the remote list.
fn add_remote(state: &mut AppState, add: GitRemoteAddState) {
    let name = add.fields.first().trim().to_string();
    let url = add.fields.second().trim().to_string();
    state.dialogs.replace(*add.previous_popup);
    state.run_git_local(
        &add.repo_path,
        move |repo| {
            let added = crate::git::remote::add_remote(repo, &name, &url);
            then_list_remotes(repo, added.ctx("git_error_add_remote_failed"))
        },
        show_remotes,
    );
}

/// Job side of a remote change: the remote list after `changed` succeeded.
pub(super) fn then_list_remotes(
    repo: &git2::Repository,
    changed: Result<(), GitFailure>,
) -> Result<Vec<RemoteInfo>, GitFailure> {
    changed.map(|()| crate::git::remote::list_remotes(repo).unwrap_or_default())
}

/// UI side of a remote change: the remote manager on top shows `remotes`
/// (otherwise the Git panel is re-read).
pub(super) fn show_remotes(state: &mut AppState, remotes: Vec<RemoteInfo>) {
    match state.dialogs.top_mut() {
        Some(PopupType::GitPrompt(GitPromptPopup::RemoteManage(manage))) => {
            manage.selected_idx = manage.selected_idx.min(remotes.len().saturating_sub(1));
            manage.remotes = remotes;
        }
        Some(PopupType::GitPanel(panel)) => {
            let repo_path = panel.repo_path.clone();
            state.reload_git_panel(&repo_path);
        }
        _ => {}
    }
}
