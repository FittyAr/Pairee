//! Handles key input for the generic GitConfirmAction dialog.

use super::common::restore_previous_and_refresh;
use crate::app::context::AppContext;
use crate::app::state::popup::GitPromptPopup;
use crate::app::state::types::GitConfirmedAction;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_confirm_action(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitPrompt(GitPromptPopup::ConfirmAction(action_state))) =
        state.dialogs.top().cloned()
    {
        let repo_path = action_state.repo_path;
        let action = action_state.action;
        let previous_popup = action_state.previous_popup;
        match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                match action {
                    GitConfirmedAction::DeleteBranch(name) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::branches::delete_branch(&repo, &name) {
                                Ok(_) => {
                                    restore_previous_and_refresh(state, *previous_popup, &repo_path)
                                }
                                Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                    "Delete branch failed: {}",
                                    e
                                ))),
                            }
                        }
                    }
                    GitConfirmedAction::MergeBranch(name) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::merge::merge(&repo, &name) {
                                Ok(_analysis) => {
                                    restore_previous_and_refresh(
                                        state,
                                        *previous_popup,
                                        &repo_path,
                                    );
                                    let has_conflicts = repo
                                        .index()
                                        .map(|idx| idx.has_conflicts())
                                        .unwrap_or(false);
                                    if has_conflicts {
                                        state.dialogs.replace(PopupType::Error("Merge conflicts detected! Please resolve them manually.".to_string()));
                                    } else {
                                        state.dialogs.replace(PopupType::Info(
                                            "Merge completed successfully.".to_string(),
                                        ));
                                    }
                                }
                                Err(e) => state
                                    .dialogs
                                    .replace(PopupType::Error(format!("Merge failed: {}", e))),
                            }
                        }
                    }
                    GitConfirmedAction::StashDrop(index) => {
                        if let Some(mut repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::stash::stash_drop(&mut repo, index) {
                                Ok(_) => {
                                    restore_previous_and_refresh(state, *previous_popup, &repo_path)
                                }
                                Err(e) => state
                                    .dialogs
                                    .replace(PopupType::Error(format!("Stash drop failed: {}", e))),
                            }
                        }
                    }
                    GitConfirmedAction::StashPop(index) => {
                        if let Some(mut repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::stash::stash_pop(&mut repo, index) {
                                Ok(_) => {
                                    restore_previous_and_refresh(state, *previous_popup, &repo_path)
                                }
                                Err(e) => state
                                    .dialogs
                                    .replace(PopupType::Error(format!("Stash pop failed: {}", e))),
                            }
                        }
                    }
                    GitConfirmedAction::ResetCommit(hash, mode) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::reset::reset(&repo, &hash, mode) {
                                Ok(_) => {
                                    restore_previous_and_refresh(state, *previous_popup, &repo_path)
                                }
                                Err(e) => state
                                    .dialogs
                                    .replace(PopupType::Error(format!("Reset failed: {}", e))),
                            }
                        }
                    }
                }
                return Ok(None);
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                state.dialogs.replace(*previous_popup);
                return Ok(None);
            }
            _ => {}
        }
        Ok(None)
    } else {
        Err(())
    }
}
