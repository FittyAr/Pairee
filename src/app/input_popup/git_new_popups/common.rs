//! Closing a Git dialog: back to the dialog under it (usually the Git
//! panel, which is refreshed).

use crate::app::git_local::GitFailure;
use crate::app::state::{AppState, GitPanelState, PopupType};
use std::path::Path;

/// Ends a Git dialog whose operation changes the working tree (commit,
/// checkout): the dialog closes at once (back to the one under it), `work`
/// runs in the background and returns the message to show; then the file
/// panels and the Git panel are re-read and the message opens on top.
pub fn run_and_report<W>(state: &mut AppState, repo_path: &Path, show_hidden: bool, work: W)
where
    W: FnOnce(&mut git2::Repository) -> Result<PopupType, GitFailure> + Send + 'static,
{
    state.dialogs.pop();
    let reload = repo_path.to_path_buf();
    state.run_git_local(repo_path, work, move |state, popup| {
        state.refresh_both_panels(show_hidden);
        state.reload_git_panel(&reload);
        state.dialogs.push(popup);
    });
}

/// Closes the Git dialog on top: back to the dialog under it, refreshed
/// when it is the Git panel.
pub fn close_to(state: &mut AppState, repo_path: &Path) {
    state.dialogs.pop();
    refresh_panel_on_top(state, repo_path);
}

/// A Git panel on top is re-read in the background, keeping its tab and
/// cursor (an error replaces it if the repository is gone).
pub fn refresh_panel_on_top(state: &mut AppState, repo_path: &Path) {
    if let Some(panel) = state.dialogs.top_as::<GitPanelState>() {
        let (tab, cursor) = (panel.active_tab, panel.cursor_idx);
        state.refresh_git_panel(repo_path, tab, cursor);
    }
}
