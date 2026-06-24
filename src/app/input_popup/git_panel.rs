use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handles keyboard input for the main Git panel popup.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::GitPanel {
        mut active_tab,
        mut cursor_idx,
        mut scroll,
        status_entries,
        log_entries,
        branch_entries,
        repo_path,
        current_branch,
        ..
    }) = state.active_popup.clone()
    {
        let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);

        let current_list_len = match active_tab {
            0 => status_entries.len(),
            1 => log_entries.len(),
            2 => branch_entries.len(),
            _ => 0,
        };

        match key.code {
            // ── Tab navigation ───────────────────────────────────────────────
            KeyCode::Tab if !is_shift => {
                active_tab = (active_tab + 1) % 3;
                cursor_idx = 0;
                scroll = 0;
            }
            KeyCode::BackTab | KeyCode::Tab if is_shift => {
                active_tab = if active_tab == 0 { 2 } else { active_tab - 1 };
                cursor_idx = 0;
                scroll = 0;
            }

            // ── Cursor movement ──────────────────────────────────────────────
            KeyCode::Up => {
                if cursor_idx > 0 {
                    cursor_idx -= 1;
                }
            }
            KeyCode::Down => {
                if current_list_len > 0 && cursor_idx < current_list_len - 1 {
                    cursor_idx += 1;
                }
            }
            KeyCode::PageUp => {
                cursor_idx = cursor_idx.saturating_sub(10);
            }
            KeyCode::PageDown => {
                cursor_idx = (cursor_idx + 10).min(current_list_len.saturating_sub(1));
            }
            KeyCode::Home => {
                cursor_idx = 0;
            }
            KeyCode::End => {
                cursor_idx = current_list_len.saturating_sub(1);
            }

            // ── Commit (Tab 0 Status) ────────────────────────────────────────
            KeyCode::Char('c') | KeyCode::Char('C') if active_tab == 0 => {
                state.active_popup = Some(PopupType::GitCommitPrompt {
                    input: String::new(),
                    cursor_idx: 0,
                    repo_path,
                });
                return Ok(None);
            }

            // ── Enter: checkout branch or commit ─────────────────────────────
            KeyCode::Enter => {
                match active_tab {
                    1 => {
                        // Log tab — checkout commit
                        if let Some(commit) = log_entries.get(cursor_idx) {
                            state.active_popup = Some(PopupType::GitConfirmCheckout {
                                target: commit.hash_full.clone(),
                                is_branch: false,
                                repo_path,
                            });
                            return Ok(None);
                        }
                    }
                    2 => {
                        // Branches tab — checkout branch (local only)
                        if let Some(branch) = branch_entries.get(cursor_idx) {
                            if !branch.is_remote {
                                state.active_popup = Some(PopupType::GitConfirmCheckout {
                                    target: branch.name.clone(),
                                    is_branch: true,
                                    repo_path,
                                });
                                return Ok(None);
                            }
                        }
                    }
                    _ => {}
                }
            }

            // ── Refresh ──────────────────────────────────────────────────────
            KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::F(5) => {
                if let Some(repo) = crate::git::repo::find_repo(&repo_path) {
                    let new_branch = repo
                        .head()
                        .ok()
                        .and_then(|h| h.shorthand().map(|s| s.to_string()))
                        .unwrap_or_else(|| "(detached HEAD)".to_string());
                    state.active_popup = Some(PopupType::GitPanel {
                        repo_path: repo_path.clone(),
                        active_tab,
                        cursor_idx: 0,
                        scroll: 0,
                        status_entries: crate::git::status::get_status(&repo),
                        log_entries: crate::git::log::get_log(&repo, 100),
                        branch_entries: crate::git::branches::get_branches(&repo),
                        current_branch: new_branch,
                        pending_action: None,
                    });
                }
                return Ok(None);
            }

            // ── Close ────────────────────────────────────────────────────────
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                state.active_popup = None;
                return Ok(None);
            }

            _ => return Ok(None),
        }

        // Update scroll so cursor stays in view
        if cursor_idx < scroll {
            scroll = cursor_idx;
        }

        state.active_popup = Some(PopupType::GitPanel {
            repo_path,
            active_tab,
            cursor_idx,
            scroll,
            status_entries,
            log_entries,
            branch_entries,
            current_branch,
            pending_action: None,
        });
        Ok(None)
    } else {
        Err(())
    }
}
