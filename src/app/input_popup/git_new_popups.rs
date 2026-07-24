use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::app::state::types::GitConfirmedAction;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

/// Restores the previous popup state (usually GitPanel) and refreshes its lists.
fn restore_previous_and_refresh(state: &mut AppState, previous: PopupType, repo_path: &std::path::Path) {
    if let PopupType::GitPanel { active_tab, cursor_idx, .. } = previous {
        if let Some(mut repo) = crate::git::repo::find_repo(repo_path) {
            let new_branch = repo
                .head()
                .ok()
                .and_then(|h| h.shorthand().ok().map(|s| s.to_string()))
                .unwrap_or_else(|| "(detached HEAD)".to_string());

            let status_entries = crate::git::status::get_status(&repo);
            let log_entries = crate::git::log::get_log(&repo, 100);
            let branch_entries = crate::git::branches::get_branches(&repo);
            let stash_entries = crate::git::stash::list_stashes(&mut repo).unwrap_or_default();

            let list_len = match active_tab {
                0 => status_entries.len(),
                1 => log_entries.len(),
                2 => branch_entries.len(),
                3 => stash_entries.len(),
                _ => 0,
            };
            let safe_cursor = cursor_idx.min(list_len.saturating_sub(1));

            state.active_popup = Some(PopupType::GitPanel {
                repo_path: repo_path.to_path_buf(),
                active_tab,
                cursor_idx: safe_cursor,
                scroll: 0,
                status_entries,
                log_entries,
                branch_entries,
                stash_entries,
                current_branch: new_branch,
                pending_action: None,
            });
        } else {
            state.active_popup = None;
        }
    } else {
        state.active_popup = Some(previous);
    }
}

/// Handles key input for the GitDiffView popup.
pub fn handle_diff(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitDiffView {
        repo_path,
        file_path,
        commit_hash,
        diff_content,
        mut scroll_y,
        previous_popup,
    }) = state.active_popup.clone()
    {
        let lines_count = diff_content.lines().count();

        match key.code {
            KeyCode::Up => {
                if scroll_y > 0 {
                    scroll_y -= 1;
                }
            }
            KeyCode::Down => {
                if scroll_y + 5 < lines_count {
                    scroll_y += 1;
                }
            }
            KeyCode::PageUp => {
                scroll_y = scroll_y.saturating_sub(15);
            }
            KeyCode::PageDown => {
                scroll_y = (scroll_y + 15).min(lines_count.saturating_sub(5));
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                state.active_popup = Some(*previous_popup);
                return Ok(None);
            }
            _ => {}
        }

        state.active_popup = Some(PopupType::GitDiffView {
            repo_path,
            file_path,
            commit_hash,
            diff_content,
            scroll_y,
            previous_popup,
        });
        Ok(None)
    } else {
        Err(())
    }
}

/// Handles key input for prompts like branch creation, branch renaming, and stash saving.
pub fn handle_prompt(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.active_popup.clone();
    if let Some(p) = popup {
        match p {
            PopupType::GitBranchCreatePrompt {
                mut input,
                mut cursor_idx, // 0 = Input, 1 = OK, 2 = Cancel
                repo_path,
                previous_popup,
            } => {
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
                            state.active_popup = Some(*previous_popup);
                            return Ok(None);
                        }
                        if !input.trim().is_empty() {
                            if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                                match crate::git::branches::create_branch(&repo, &input, "HEAD") {
                                    Ok(_) => restore_previous_and_refresh(state, *previous_popup, &repo_path),
                                    Err(e) => state.active_popup = Some(PopupType::Error(format!("Failed to create branch: {}", e))),
                                }
                            }
                        } else {
                            state.active_popup = Some(*previous_popup);
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = Some(*previous_popup);
                        return Ok(None);
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::GitBranchCreatePrompt {
                    input,
                    cursor_idx,
                    repo_path,
                    previous_popup,
                });
            }

            PopupType::GitBranchRenamePrompt {
                mut input,
                mut cursor_idx, // 0 = Input, 1 = OK, 2 = Cancel
                old_name,
                repo_path,
                previous_popup,
            } => {
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
                            state.active_popup = Some(*previous_popup);
                            return Ok(None);
                        }
                        if !input.trim().is_empty() && input != old_name {
                            if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                                match crate::git::branches::rename_branch(&repo, &old_name, &input) {
                                    Ok(_) => restore_previous_and_refresh(state, *previous_popup, &repo_path),
                                    Err(e) => state.active_popup = Some(PopupType::Error(format!("Failed to rename branch: {}", e))),
                                }
                            }
                        } else {
                            state.active_popup = Some(*previous_popup);
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = Some(*previous_popup);
                        return Ok(None);
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::GitBranchRenamePrompt {
                    input,
                    cursor_idx,
                    old_name,
                    repo_path,
                    previous_popup,
                });
            }

            PopupType::GitStashSavePrompt {
                mut input,
                mut cursor_idx, // 0 = Input, 1 = OK, 2 = Cancel
                repo_path,
                previous_popup,
            } => {
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
                            state.active_popup = Some(*previous_popup);
                            return Ok(None);
                        }
                        let msg = if input.trim().is_empty() { None } else { Some(input.as_str()) };
                        if let Some(mut repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::stash::stash_save(&mut repo, msg, true) {
                                Ok(_) => restore_previous_and_refresh(state, *previous_popup, &repo_path),
                                Err(e) => state.active_popup = Some(PopupType::Error(format!("Stash save failed: {}", e))),
                            }
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = Some(*previous_popup);
                        return Ok(None);
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::GitStashSavePrompt {
                    input,
                    cursor_idx,
                    repo_path,
                    previous_popup,
                });
            }
            _ => return Err(()),
        }
        Ok(None)
    } else {
        Err(())
    }
}

/// Handles key input for the generic GitConfirmAction dialog.
pub fn handle_confirm_action(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitConfirmAction {
        message: _,
        repo_path,
        action,
        previous_popup,
    }) = state.active_popup.clone()
    {
        // 0 = OK / Yes, 1 = Cancel / No. Let's make Enter confirm, and arrow navigation.
        // We will hold focus state on a small selection state or just simple Enter / Esc.
        // For simplicity: Yes on Enter/y/Y, No on Esc/n/N/BackTab.
        match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                match action {
                    GitConfirmedAction::DeleteBranch(name) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::branches::delete_branch(&repo, &name) {
                                Ok(_) => restore_previous_and_refresh(state, *previous_popup, &repo_path),
                                Err(e) => state.active_popup = Some(PopupType::Error(format!("Delete branch failed: {}", e))),
                            }
                        }
                    }
                    GitConfirmedAction::MergeBranch(name) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::merge::merge(&repo, &name) {
                                Ok(_analysis) => {
                                    restore_previous_and_refresh(state, *previous_popup, &repo_path);
                                    let has_conflicts = repo.index().map(|idx| idx.has_conflicts()).unwrap_or(false);
                                    if has_conflicts {
                                        state.active_popup = Some(PopupType::Error("Merge conflicts detected! Please resolve them manually.".to_string()));
                                    } else {
                                        state.active_popup = Some(PopupType::Info("Merge completed successfully.".to_string()));
                                    }
                                }
                                Err(e) => state.active_popup = Some(PopupType::Error(format!("Merge failed: {}", e))),
                            }
                        }
                    }
                    GitConfirmedAction::StashDrop(index) => {
                        if let Some(mut repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::stash::stash_drop(&mut repo, index) {
                                Ok(_) => restore_previous_and_refresh(state, *previous_popup, &repo_path),
                                Err(e) => state.active_popup = Some(PopupType::Error(format!("Stash drop failed: {}", e))),
                            }
                        }
                    }
                    GitConfirmedAction::StashPop(index) => {
                        if let Some(mut repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::stash::stash_pop(&mut repo, index) {
                                Ok(_) => restore_previous_and_refresh(state, *previous_popup, &repo_path),
                                Err(e) => state.active_popup = Some(PopupType::Error(format!("Stash pop failed: {}", e))),
                            }
                        }
                    }
                    GitConfirmedAction::ResetCommit(hash, mode) => {
                        if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                            match crate::git::reset::reset(&repo, &hash, mode) {
                                Ok(_) => restore_previous_and_refresh(state, *previous_popup, &repo_path),
                                Err(e) => state.active_popup = Some(PopupType::Error(format!("Reset failed: {}", e))),
                            }
                        }
                    }
                }
                return Ok(None);
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                state.active_popup = Some(*previous_popup);
                return Ok(None);
            }
            _ => {}
        }
        Ok(None)
    } else {
        Err(())
    }
}
