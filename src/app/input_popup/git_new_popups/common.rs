//! Restores the previous popup state (usually GitPanel) and refreshes its lists.

use crate::app::git_local::GitFailure;
use crate::app::state::{AppState, PopupType};

/// Ends a Git dialog whose operation changes the working tree (commit,
/// checkout): the dialog closes at once (back to `previous`, or no dialog),
/// `work` runs in the background and returns the message to show; then the
/// file panels and the Git panel are re-read and the message opens on top.
pub fn run_and_report<W>(
    state: &mut AppState,
    previous: Option<Box<PopupType>>,
    repo_path: &std::path::Path,
    show_hidden: bool,
    work: W,
) where
    W: FnOnce(&mut git2::Repository) -> Result<PopupType, GitFailure> + Send + 'static,
{
    match previous {
        Some(previous) => state.dialogs.replace(*previous),
        None => state.dialogs.clear(),
    }
    let reload = repo_path.to_path_buf();
    state.run_git_local(repo_path, work, move |state, popup| {
        state.refresh_both_panels(show_hidden);
        state.reload_git_panel(&reload);
        state.dialogs.push(popup);
    });
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
