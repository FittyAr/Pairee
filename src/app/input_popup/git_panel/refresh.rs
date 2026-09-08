//! Git repository state refresher for the popup panel.

use crate::app::state::{AppState, PopupType};
use std::path::Path;

pub fn refresh_git_panel(
    state: &mut AppState,
    repo_path: &Path,
    active_tab: usize,
    cursor_idx: usize,
) {
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

        state
            .dialogs
            .replace(PopupType::GitPanel(crate::app::state::GitPanelState {
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
            }));
    }
}
