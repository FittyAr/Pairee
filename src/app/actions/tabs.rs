//! Folder tab actions on the focused side.

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;

/// Handles the tab actions. Returns `true` if the action was one of them.
pub fn handle_tab_action(state: &mut AppState, action: &Action, context: &AppContext) -> bool {
    let show_hidden = context.config.settings.show_hidden;
    match *action {
        Action::NewTab => state.duplicate_active_tab(show_hidden),
        Action::CloseTab => {
            state.close_active_tab(show_hidden);
        }
        Action::NextTab | Action::PrevTab => {
            state.cycle_tab(*action == Action::NextTab, show_hidden);
        }
        Action::GoToTab(n) => {
            state.activate_tab_index(usize::from(n).saturating_sub(1), show_hidden);
        }
        Action::MoveTabLeft | Action::MoveTabRight => {
            state.move_active_tab(*action == Action::MoveTabRight);
        }
        Action::ToggleTabLock => state.toggle_active_tab_lock(),
        Action::RenameTab => open_rename_prompt(state),
        Action::OpenInNewTab => {
            crate::app::input::handle_open_in_new_tab_key(state, context);
        }
        _ => return false,
    }
    true
}

fn open_rename_prompt(state: &mut AppState) {
    let tab = state.panels.tabs(state.panels.active).active();
    state.dialogs.replace(PopupType::RenameTabPrompt {
        tab: tab.id,
        input: tab.name.clone().unwrap_or_default().into(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::ActivePanel;
    use crate::config::AppConfig;

    fn run(state: &mut AppState, context: &AppContext, action: Action) {
        assert!(handle_tab_action(state, &action, context), "{action:?}");
    }

    #[test]
    fn tab_actions_drive_the_focused_side() {
        let context = AppContext::new(AppConfig::default());
        let dir = tempfile::tempdir().expect("tempdir");
        let mut state = AppState::new(dir.path().into(), dir.path().into());
        state.panels.active = ActivePanel::Right;
        run(&mut state, &context, Action::NewTab);
        run(&mut state, &context, Action::NewTab);
        let tabs = state.panels.tabs(ActivePanel::Right);
        assert_eq!((tabs.count(), tabs.active_index()), (3, 2));
        assert_eq!(state.panels.tabs(ActivePanel::Left).count(), 1);
        run(&mut state, &context, Action::GoToTab(1));
        run(&mut state, &context, Action::MoveTabRight);
        run(&mut state, &context, Action::NextTab);
        assert_eq!(state.panels.tabs(ActivePanel::Right).active_index(), 2);
        run(&mut state, &context, Action::ToggleTabLock);
        assert!(
            state
                .panels
                .tabs(ActivePanel::Right)
                .active()
                .lock
                .is_some()
        );
        run(&mut state, &context, Action::RenameTab);
        assert!(matches!(
            state.dialogs.top(),
            Some(PopupType::RenameTabPrompt { .. })
        ));
        run(&mut state, &context, Action::CloseTab);
        assert_eq!(state.panels.tabs(ActivePanel::Right).count(), 2);
        assert!(!handle_tab_action(&mut state, &Action::Copy, &context));
    }

    #[test]
    fn open_in_new_tab_enters_the_folder_under_the_cursor() {
        let context = AppContext::new(AppConfig::default());
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("inner")).unwrap();
        let mut state = AppState::new(dir.path().into(), dir.path().into());
        state.refresh_active_panel(false);
        let panel = state.get_active_panel_mut();
        panel.cursor_index = panel
            .entries
            .iter()
            .position(|e| e.name == "inner")
            .unwrap();
        run(&mut state, &context, Action::OpenInNewTab);
        let tabs = state.panels.tabs(ActivePanel::Left);
        assert_eq!(tabs.count(), 2);
        assert_eq!(tabs.panel().current_path, dir.path().join("inner"));
        assert_eq!(tabs.tabs()[0].panel.current_path, dir.path());
    }
}
