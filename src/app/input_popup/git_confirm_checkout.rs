use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

/// Handles keyboard input for the git checkout confirmation dialog.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitConfirmCheckout {
        target,
        is_branch,
        repo_path,
    }) = state.active_popup.clone()
    {
        match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                    let result = if is_branch {
                        crate::git::checkout::checkout_branch(&repo, &target)
                    } else {
                        crate::git::checkout::checkout_commit(&repo, &target)
                    };
                    match result {
                        Ok(()) => {
                            state.active_popup = Some(PopupType::Info(format!(
                                "{}: {}",
                                crate::config::localization::t("git_checkout_success"),
                                target
                            )));
                        }
                        Err(e) => {
                            state.active_popup = Some(PopupType::Error(format!(
                                "{}: {}",
                                crate::config::localization::t("git_checkout_error"),
                                e
                            )));
                        }
                    }
                } else {
                    state.active_popup = Some(PopupType::Error(
                        crate::config::localization::t("git_not_a_repo"),
                    ));
                }
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                state.active_popup = None;
            }
            _ => return Ok(None),
        }
        Ok(None)
    } else {
        Err(())
    }
}
