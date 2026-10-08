//! Restores the previous popup state (usually GitPanel) and refreshes its lists.

use crate::app::state::{AppState, PopupType};

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
