//! Git repository control panel popup handler.

mod refresh;
mod remote;
mod tabs;

pub use refresh::refresh_git_panel;

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
    if let Some(PopupType::GitPanel(crate::app::state::GitPanelState {
        mut active_tab,
        mut cursor_idx,
        mut scroll,
        status_entries,
        log_entries,
        branch_entries,
        stash_entries,
        repo_path,
        current_branch,
        ..
    })) = state.dialogs.top().cloned()
    {
        let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);

        let current_list_len = match active_tab {
            0 => status_entries.len(),
            1 => log_entries.len(),
            2 => branch_entries.len(),
            3 => stash_entries.len(),
            _ => 0,
        };

        match key.code {
            // ── Tab navigation (4 tabs) ──────────────────────────────────────
            KeyCode::Tab if !is_shift => {
                active_tab = (active_tab + 1) % 4;
                cursor_idx = 0;
                scroll = 0;
            }
            KeyCode::BackTab | KeyCode::Tab if is_shift => {
                active_tab = if active_tab == 0 { 3 } else { active_tab - 1 };
                cursor_idx = 0;
                scroll = 0;
            }

            // ── Cursor movement ──────────────────────────────────────────────
            KeyCode::Up => {
                cursor_idx = cursor_idx.saturating_sub(1);
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

            // ── Global Remote Actions (Fetch / Pull / Push) ──────────────────
            KeyCode::Char('f') | KeyCode::Char('F') => {
                remote::handle_fetch(state, &repo_path);
                return Ok(None);
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                remote::handle_pull(state, &repo_path, active_tab, cursor_idx);
                return Ok(None);
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                remote::handle_push(state, &repo_path);
                return Ok(None);
            }

            // ── Tab 0 (Status) Actions ───────────────────────────────────────
            _ if active_tab == 0
                && tabs::handle_status_tab(
                    state,
                    key.code,
                    &repo_path,
                    &status_entries,
                    cursor_idx,
                ) =>
            {
                return Ok(None);
            }

            // ── Tab 1 (Log) Actions ──────────────────────────────────────────
            _ if active_tab == 1
                && tabs::handle_log_tab(state, key.code, &repo_path, &log_entries, cursor_idx) =>
            {
                return Ok(None);
            }

            // ── Tab 2 (Branches) Actions ─────────────────────────────────────
            _ if active_tab == 2
                && tabs::handle_branch_tab(
                    state,
                    key.code,
                    &repo_path,
                    &branch_entries,
                    cursor_idx,
                    &current_branch,
                ) =>
            {
                return Ok(None);
            }

            // ── Tab 3 (Stash) Actions ────────────────────────────────────────
            _ if active_tab == 3
                && tabs::handle_stash_tab(
                    state,
                    key.code,
                    &repo_path,
                    &stash_entries,
                    cursor_idx,
                ) =>
            {
                return Ok(None);
            }

            // ── Refresh ──────────────────────────────────────────────────────
            KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::F(5) => {
                refresh_git_panel(state, &repo_path, active_tab, cursor_idx);
                return Ok(None);
            }

            // ── Close ────────────────────────────────────────────────────────
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                state.dialogs.clear();
                return Ok(None);
            }

            _ => return Ok(None),
        }

        // Update scroll so cursor stays in view
        if cursor_idx < scroll {
            scroll = cursor_idx;
        }

        state
            .dialogs
            .replace(PopupType::GitPanel(crate::app::state::GitPanelState {
                repo_path,
                active_tab,
                cursor_idx,
                scroll,
                status_entries,
                log_entries,
                branch_entries,
                stash_entries,
                current_branch,
                pending_action: None,
            }));
        Ok(None)
    } else {
        Err(())
    }
}
