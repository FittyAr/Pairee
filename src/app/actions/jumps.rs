//! Folder jumps of the active panel: back / forward through its history,
//! home and root.

use crate::app::context::AppContext;
use crate::app::session::restore::home_dir;
use crate::app::state::AppState;
use crate::app::state::nav_history::Travel;
use crate::keybindings::Action;
use std::path::Path;

pub fn handle_jump_action(state: &mut AppState, action: &Action, context: &AppContext) -> bool {
    let show_hidden = context.config.settings.show_hidden;
    match action {
        Action::HistoryBack => travel(state, Travel::Back, show_hidden),
        Action::HistoryForward => travel(state, Travel::Forward, show_hidden),
        Action::GoHome => state.jump_active_panel_to(home_dir(), show_hidden),
        Action::GoRoot => go_root(state, show_hidden),
        _ => return false,
    }
    true
}

fn travel(state: &mut AppState, way: Travel, show_hidden: bool) {
    let panel = state.get_active_panel_mut();
    let current = panel.current_path.clone();
    if let Some(target) = panel.nav.travel(way, current) {
        panel.open_path(target);
        state.refresh_active_panel(show_hidden);
    }
}

/// The root of the panel's drive or filesystem (on the same source).
fn go_root(state: &mut AppState, show_hidden: bool) {
    let panel = state.get_active_panel_mut();
    let Some(root) = panel.current_path.ancestors().last().map(Path::to_path_buf) else {
        return;
    };
    if root != panel.current_path {
        panel.open_path(root);
        state.refresh_active_panel(show_hidden);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    #[test]
    fn back_and_forward_follow_visited_folders() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let mut state = AppState::new(dir.path().to_path_buf(), dir.path().to_path_buf());
        let context = AppContext::new(AppConfig::default());
        state.get_active_panel_mut().open_path(sub.clone());
        state.refresh_active_panel(false);
        assert!(handle_jump_action(
            &mut state,
            &Action::HistoryBack,
            &context
        ));
        assert_eq!(state.get_active_panel().current_path, dir.path());
        handle_jump_action(&mut state, &Action::HistoryForward, &context);
        assert_eq!(state.get_active_panel().current_path, sub);
    }

    #[test]
    fn root_is_the_first_ancestor() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = AppState::new(dir.path().to_path_buf(), dir.path().to_path_buf());
        let context = AppContext::new(AppConfig::default());
        handle_jump_action(&mut state, &Action::GoRoot, &context);
        let root = state.get_active_panel().current_path.clone();
        assert_eq!(root.parent(), None, "{root:?}");
    }
}
