use super::common::restore_previous_and_refresh;
use crate::app::context::AppContext;
use crate::app::state::popup::{
    GitBranchCreatePromptState, GitBranchRenamePromptState, GitPromptPopup, GitStashSavePromptState,
};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_prompt(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.dialogs.top().cloned();
    if let Some(p) = popup {
        match p {
            PopupType::GitPrompt(GitPromptPopup::BranchCreatePrompt(
                GitBranchCreatePromptState {
                    mut input,
                    mut cursor_idx,
                    repo_path,
                    previous_popup,
                },
            )) => {
                match key.code {
                    KeyCode::Up | KeyCode::BackTab => {
                        cursor_idx = if cursor_idx > 0 { cursor_idx - 1 } else { 2 };
                    }
                    KeyCode::Down | KeyCode::Tab => {
                        cursor_idx = (cursor_idx + 1) % 3;
                    }
                    KeyCode::Char(c) if cursor_idx == 0 => {
                        input.push(c);
                    }
                    KeyCode::Backspace if cursor_idx == 0 => {
                        input.pop();
                    }
                    KeyCode::Enter => {
                        if cursor_idx == 2 {
                            state.dialogs.replace(*previous_popup);
                            return Ok(None);
                        }
                        if !input.trim().is_empty() {
                            if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                                match crate::git::branches::create_branch(&repo, &input, "HEAD") {
                                    Ok(_) => restore_previous_and_refresh(
                                        state,
                                        *previous_popup,
                                        &repo_path,
                                    ),
                                    Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                        "Failed to create branch: {}",
                                        e
                                    ))),
                                }
                            }
                        } else {
                            state.dialogs.replace(*previous_popup);
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.dialogs.replace(*previous_popup);
                        return Ok(None);
                    }
                    _ => {}
                }
                state
                    .dialogs
                    .replace(PopupType::GitPrompt(GitPromptPopup::BranchCreatePrompt(
                        GitBranchCreatePromptState {
                            input,
                            cursor_idx,
                            repo_path,
                            previous_popup,
                        },
                    )));
            }

            PopupType::GitPrompt(GitPromptPopup::BranchRenamePrompt(
                GitBranchRenamePromptState {
                    mut input,
                    mut cursor_idx,
                    old_name,
                    repo_path,
                    previous_popup,
                },
            )) => {
                match key.code {
                    KeyCode::Up | KeyCode::BackTab => {
                        cursor_idx = if cursor_idx > 0 { cursor_idx - 1 } else { 2 };
                    }
                    KeyCode::Down | KeyCode::Tab => {
                        cursor_idx = (cursor_idx + 1) % 3;
                    }
                    KeyCode::Char(c) if cursor_idx == 0 => {
                        input.push(c);
                    }
                    KeyCode::Backspace if cursor_idx == 0 => {
                        input.pop();
                    }
                    KeyCode::Enter => {
                        if cursor_idx == 2 {
                            state.dialogs.replace(*previous_popup);
                            return Ok(None);
                        }
                        if !input.trim().is_empty() && input != old_name {
                            if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                                match crate::git::branches::rename_branch(&repo, &old_name, &input)
                                {
                                    Ok(_) => restore_previous_and_refresh(
                                        state,
                                        *previous_popup,
                                        &repo_path,
                                    ),
                                    Err(e) => state.dialogs.replace(PopupType::Error(format!(
                                        "Failed to rename branch: {}",
                                        e
                                    ))),
                                }
                            }
                        } else {
                            state.dialogs.replace(*previous_popup);
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.dialogs.replace(*previous_popup);
                        return Ok(None);
                    }
                    _ => {}
                }
                state
                    .dialogs
                    .replace(PopupType::GitPrompt(GitPromptPopup::BranchRenamePrompt(
                        GitBranchRenamePromptState {
                            input,
                            cursor_idx,
                            old_name,
                            repo_path,
                            previous_popup,
                        },
                    )));
            }

            PopupType::GitPrompt(GitPromptPopup::StashSavePrompt(GitStashSavePromptState {
                mut input,
                mut cursor_idx,
                repo_path,
                previous_popup,
            })) => {
                match key.code {
                    KeyCode::Up | KeyCode::BackTab => {
                        cursor_idx = if cursor_idx > 0 { cursor_idx - 1 } else { 2 };
                    }
                    KeyCode::Down | KeyCode::Tab => {
                        cursor_idx = (cursor_idx + 1) % 3;
                    }
                    KeyCode::Char(c) if cursor_idx == 0 => {
                        input.push(c);
                    }
                    KeyCode::Backspace if cursor_idx == 0 => {
                        input.pop();
                    }
                    KeyCode::Enter => {
                        if cursor_idx == 2 {
                            state.dialogs.replace(*previous_popup);
                            return Ok(None);
                        }
                        let msg = if input.trim().is_empty() {
                            None
                        } else {
                            Some(input.as_str())
                        };
                        if let Some(mut repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::stash::stash_save(&mut repo, msg, true) {
                                Ok(_) => {
                                    restore_previous_and_refresh(state, *previous_popup, &repo_path)
                                }
                                Err(e) => state
                                    .dialogs
                                    .replace(PopupType::Error(format!("Stash save failed: {}", e))),
                            }
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.dialogs.replace(*previous_popup);
                        return Ok(None);
                    }
                    _ => {}
                }
                state
                    .dialogs
                    .replace(PopupType::GitPrompt(GitPromptPopup::StashSavePrompt(
                        GitStashSavePromptState {
                            input,
                            cursor_idx,
                            repo_path,
                            previous_popup,
                        },
                    )));
            }
            _ => return Err(()),
        }
        Ok(None)
    } else {
        Err(())
    }
}
