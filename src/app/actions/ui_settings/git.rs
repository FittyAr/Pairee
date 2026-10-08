use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::config::localization::t;

pub fn open_git_panel(state: &mut AppState, context: &AppContext) -> bool {
    if !context.config.settings.git_enabled {
        return false;
    }
    // The repository is read in the background; a path outside any
    // repository closes the panel with "not a repository" once known.
    let panel_path = state.get_active_panel().current_path.clone();
    let limit = context.config.settings.git_log_limit as usize;
    state.open_git_panel_at(&panel_path, limit);
    true
}

pub fn init_repo_action(state: &mut AppState, context: &AppContext) -> bool {
    if !context.config.settings.git_enabled {
        return false;
    }
    let panel_path = state.get_active_panel().current_path.clone();
    match crate::git::repo::init_repo(&panel_path) {
        Ok(_) => {
            state.refresh_both_panels(context.config.settings.show_hidden);
            state
                .dialogs
                .replace(crate::app::state::PopupType::Info(t("git_init_success")));
        }
        Err(e) => {
            state
                .dialogs
                .replace(crate::app::state::PopupType::Error(format!(
                    "{}: {}",
                    t("git_init_error"),
                    e
                )));
        }
    }
    true
}

pub fn clone_repo_prompt_action(state: &mut AppState, context: &AppContext) -> bool {
    if !context.config.settings.git_enabled {
        return false;
    }
    let target_parent_path = state.get_active_panel().current_path.clone();
    state
        .dialogs
        .replace(crate::app::state::PopupType::GitPrompt(
            crate::app::state::popup::GitPromptPopup::ClonePrompt(
                crate::app::state::popup::GitClonePromptState {
                    target_parent_path,
                    fields: Default::default(),
                },
            ),
        ));
    true
}
