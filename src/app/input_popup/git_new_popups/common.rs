//! Restores the previous popup state (usually GitPanel) and refreshes its lists.

use crate::app::state::{AppState, PopupType};

/// Ends a Git dialog: back to `previous` (refreshed) with `popup` on top, or
/// just `popup` when the dialog was opened on its own.
pub fn finish_with(
    state: &mut AppState,
    previous: Option<Box<PopupType>>,
    repo_path: &std::path::Path,
    popup: PopupType,
) {
    match previous {
        Some(previous) => {
            restore_previous_and_refresh(state, *previous, repo_path);
            state.dialogs.push(popup);
        }
        None => state.dialogs.replace(popup),
    }
}

/// Closes a Git dialog: back to `previous` (refreshed), or no dialog at all.
pub fn close_to(
    state: &mut AppState,
    previous: Option<Box<PopupType>>,
    repo_path: &std::path::Path,
) {
    match previous {
        Some(previous) => restore_previous_and_refresh(state, *previous, repo_path),
        None => state.dialogs.clear(),
    }
}

pub fn restore_previous_and_refresh(
    state: &mut AppState,
    previous: PopupType,
    repo_path: &std::path::Path,
) {
    // A Git panel comes back with its current contents and is refreshed in
    // the background (an error replaces it if the repository is gone).
    let panel_view = match &previous {
        PopupType::GitPanel(panel) => Some((panel.active_tab, panel.cursor_idx)),
        _ => None,
    };
    state.dialogs.replace(previous);
    if let Some((tab, cursor)) = panel_view {
        state.refresh_git_panel(repo_path, tab, cursor);
    }
}
