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
    if let PopupType::GitPanel(panel) = previous {
        if !crate::app::input_popup::git_panel::refresh_git_panel(
            state,
            repo_path,
            panel.active_tab,
            panel.cursor_idx,
        ) {
            state.dialogs.clear();
        }
    } else {
        state.dialogs.replace(previous);
    }
}
