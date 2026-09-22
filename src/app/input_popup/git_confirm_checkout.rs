use crate::app::context::AppContext;
use crate::app::input_popup::git_new_popups::restore_previous_and_refresh;
use crate::app::state::popup::GitPromptPopup;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

/// Handles keyboard input for the git checkout confirmation dialog.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitPrompt(GitPromptPopup::ConfirmCheckout(checkout_state))) =
        state.dialogs.top().cloned()
    {
        let target = checkout_state.target;
        let is_branch = checkout_state.is_branch;
        let repo_path = checkout_state.repo_path;
        let previous_popup = checkout_state.previous_popup;
        match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                    let result = if is_branch {
                        crate::git::checkout::checkout_branch(&repo, &target)
                    } else {
                        crate::git::checkout::checkout_commit(&repo, &target)
                            .map(|()| target.clone())
                    };
                    match result {
                        Ok(checked_out_target) => {
                            state.refresh_both_panels(context.config.settings.show_hidden);
                            if let Some(prev) = previous_popup {
                                restore_previous_and_refresh(state, *prev, &repo_path);
                                state.dialogs.push(PopupType::Info(format!(
                                    "{}: {}",
                                    crate::config::localization::t("git_checkout_success"),
                                    checked_out_target
                                )));
                            } else {
                                state.dialogs.replace(PopupType::Info(format!(
                                    "{}: {}",
                                    crate::config::localization::t("git_checkout_success"),
                                    checked_out_target
                                )));
                            }
                        }
                        Err(e) => {
                            state.dialogs.replace(PopupType::Error(format!(
                                "{}: {}",
                                crate::config::localization::t("git_checkout_error"),
                                e
                            )));
                        }
                    }
                } else {
                    state
                        .dialogs
                        .replace(PopupType::Error(crate::config::localization::t(
                            "git_not_a_repo",
                        )));
                }
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                if let Some(prev) = previous_popup {
                    restore_previous_and_refresh(state, *prev, &repo_path);
                } else {
                    state.dialogs.clear();
                }
            }
            _ => return Ok(None),
        }
        Ok(None)
    } else {
        Err(())
    }
}
