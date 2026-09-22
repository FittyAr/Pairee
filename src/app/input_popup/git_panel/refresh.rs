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
        let new_branch = if repo.head_detached().unwrap_or(false) {
            crate::config::localization::t("git_detached_head")
        } else if let Ok(head) = repo.head()
            && let Some(name) = head.shorthand().ok()
        {
            name.to_string()
        } else if let Ok(head_ref) = repo.find_reference("HEAD")
            && let Some(target) = head_ref.symbolic_target().ok().flatten()
            && let Some(b) = target.strip_prefix("refs/heads/")
        {
            b.to_string()
        } else {
            crate::config::localization::t("git_detached_head")
        };

        let status_entries = crate::git::status::get_status(&repo);
        let log_entries = crate::git::log::get_log(&repo, 100);
        let branch_entries = crate::git::branches::get_branches(&repo);
        let stash_entries = crate::git::stash::list_stashes(&mut repo).unwrap_or_default();
        let tag_entries = crate::git::tags::list_tags(&repo).unwrap_or_default();

        let list_len = match active_tab {
            0 => status_entries.len(),
            1 => log_entries.len(),
            2 => branch_entries.len(),
            3 => stash_entries.len(),
            4 => tag_entries.len(),
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
                tag_entries,
                current_branch: new_branch,
            }));
    }
}
