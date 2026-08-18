use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::config::localization::t;

pub fn open_git_panel(state: &mut AppState, context: &AppContext) -> bool {
    if !context.config.settings.git_enabled {
        return false;
    }
    let panel_path = state.get_active_panel().current_path.clone();
    match crate::git::repo::find_repo(&panel_path) {
        Some(mut repo) => {
            let repo_path =
                crate::git::repo::get_workdir(&repo).unwrap_or_else(|| panel_path.clone());
            let current_branch = repo
                .head()
                .ok()
                .and_then(|h| h.shorthand().ok().map(|s| s.to_string()))
                .unwrap_or_else(|| t("git_detached_head"));
            let limit = context.config.settings.git_log_limit as usize;
            let status_entries = crate::git::status::get_status(&repo);
            let log_entries = crate::git::log::get_log(&repo, limit);
            let branch_entries = crate::git::branches::get_branches(&repo);
            let stash_entries = crate::git::stash::list_stashes(&mut repo).unwrap_or_default();
            state
                .dialogs
                .replace(crate::app::state::PopupType::GitPanel {
                    repo_path,
                    active_tab: 0,
                    cursor_idx: 0,
                    scroll: 0,
                    status_entries,
                    log_entries,
                    branch_entries,
                    stash_entries,
                    current_branch,
                    pending_action: None,
                });
        }
        None => {
            state.dialogs.replace(crate::app::state::PopupType::Error(
                crate::config::localization::t("git_not_a_repo"),
            ));
        }
    }
    true
}
