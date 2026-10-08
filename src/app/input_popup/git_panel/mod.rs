//! Git repository control panel popup handler.

mod remote;
mod tabs;

use crate::app::context::AppContext;
use crate::app::list_nav::{NavStep, wrap_next, wrap_prev};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Number of Git panel tabs (Status, Log, Branches, Stash, Tags).
const TABS: usize = 5;

/// Handles keyboard input for the main Git panel popup.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::GitPanel(panel)) = state.dialogs.top_mut() else {
        return Err(());
    };
    // Tabs and cursor movement edit the panel in place.
    if navigate(panel, &key) {
        state.prefetch_git_log();
        return Ok(None);
    }
    // Actions may open dialogs over the panel, so they work on a snapshot.
    let panel = panel.clone();
    let (repo, tab, cursor) = (&panel.repo_path, panel.active_tab, panel.cursor_idx);
    match key.code {
        KeyCode::Char('f' | 'F') => remote::handle_fetch(state, repo),
        KeyCode::Char('l' | 'L') => remote::handle_pull(state, repo, tab, cursor),
        KeyCode::Char('u' | 'U') => remote::handle_push(state, repo),
        _ if tab_action(state, &panel, key.code) => {}
        KeyCode::Char('r' | 'R') | KeyCode::F(5) => {
            state.refresh_git_panel(repo, tab, cursor);
        }
        KeyCode::Esc | KeyCode::Char('q' | 'Q') => state.dialogs.clear(),
        _ => {}
    }
    Ok(None)
}

/// Tab switching and list movement. Returns `false` for other keys.
fn navigate(panel: &mut crate::app::state::GitPanelState, key: &KeyEvent) -> bool {
    let back = key.code == KeyCode::BackTab
        || (key.code == KeyCode::Tab && key.modifiers.contains(KeyModifiers::SHIFT));
    if back || key.code == KeyCode::Tab {
        panel.active_tab = if back {
            wrap_prev(panel.active_tab, TABS)
        } else {
            wrap_next(panel.active_tab, TABS)
        };
        panel.cursor_idx = 0;
        panel.scroll = 0;
        return true;
    }
    let Some(step) = NavStep::from_key(key.code) else {
        return false;
    };
    let len = panel.tab_len(panel.active_tab);
    panel.cursor_idx = step.apply_clamped(panel.cursor_idx, len);
    // Keep the cursor in view.
    panel.scroll = panel.scroll.min(panel.cursor_idx);
    true
}

/// Runs the key as an action of the active tab. Returns `true` if handled.
fn tab_action(
    state: &mut AppState,
    panel: &crate::app::state::GitPanelState,
    code: KeyCode,
) -> bool {
    let (repo, cursor) = (&panel.repo_path, panel.cursor_idx);
    match panel.active_tab {
        0 => tabs::handle_status_tab(state, code, repo, &panel.status_entries, cursor),
        1 => tabs::handle_log_tab(state, code, repo, &panel.log_entries, cursor),
        2 => tabs::handle_branch_tab(
            state,
            code,
            repo,
            &panel.branch_entries,
            cursor,
            &panel.current_branch,
        ),
        3 => tabs::handle_stash_tab(state, code, repo, &panel.stash_entries, cursor),
        4 => tabs::handle_tag_tab(state, code, repo, &panel.tag_entries, cursor),
        _ => false,
    }
}
