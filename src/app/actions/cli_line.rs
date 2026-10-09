//! Norton Commander / Far command-line and layout keys: put names or paths
//! on the command line, walk its history, clear it, hide the passive panel
//! and the key bar.

use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState};
use crate::keybindings::Action;
use std::path::Path;

pub fn handle(state: &mut AppState, action: &Action, context: &mut AppContext) -> bool {
    match action {
        Action::InsertNameToCli => {
            let panel = state.get_active_panel();
            if let Some(entry) = panel.entries.get(panel.cursor_index) {
                let name = entry.name.clone();
                insert(state, &name);
            }
        }
        Action::InsertPathToCli => {
            let panel = state.get_active_panel();
            if let Some(entry) = panel.entries.get(panel.cursor_index) {
                let path = entry.path.clone();
                insert(state, &path.to_string_lossy());
            }
        }
        Action::InsertLeftPathToCli => insert_panel_path(state, ActivePanel::Left),
        Action::InsertRightPathToCli => insert_panel_path(state, ActivePanel::Right),
        Action::ClearCli => {
            state.cli_input.clear();
            state.cli_history_pos = None;
        }
        Action::CliHistoryPrev => step_history(state, true),
        Action::CliHistoryNext => step_history(state, false),
        Action::ToggleInactivePanel => {
            let panels = &mut state.panels;
            match panels.active {
                ActivePanel::Left => panels.right_visible = !panels.right_visible,
                ActivePanel::Right => panels.left_visible = !panels.left_visible,
            }
        }
        Action::ToggleKeybar => {
            let settings = &mut context.config.settings;
            settings.interface_show_key_bar = !settings.interface_show_key_bar;
            context.config.save_logging();
        }
        _ => return false,
    }
    true
}

/// Appends `text` (quoted when it has spaces) and a space to the command line.
fn insert(state: &mut AppState, text: &str) {
    if text.contains(' ') {
        state.cli_input.push_str(&format!("\"{text}\" "));
    } else {
        state.cli_input.push_str(text);
        state.cli_input.push(' ');
    }
}

fn insert_panel_path(state: &mut AppState, side: ActivePanel) {
    let path = state.panels.side_mut(side).current_path.clone();
    insert(state, &Path::new(&path).to_string_lossy());
}

/// Puts an older (`older`) or newer command of the history on the line;
/// stepping past the newest clears it.
fn step_history(state: &mut AppState, older: bool) {
    let len = state.history.commands.len();
    let pos = match (state.cli_history_pos, older) {
        (None, true) if len > 0 => Some(0),
        (Some(i), true) => Some((i + 1).min(len.saturating_sub(1))),
        (Some(i), false) if i > 0 => Some(i - 1),
        _ => None,
    };
    state.cli_history_pos = pos;
    state.cli_input = pos
        .and_then(|i| state.history.commands.get(i).cloned())
        .unwrap_or_default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use std::path::PathBuf;

    fn run(state: &mut AppState, action: Action) {
        let mut context = AppContext::new(AppConfig::default());
        assert!(handle(state, &action, &mut context));
    }

    #[test]
    fn history_walks_back_and_forth() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.history.commands = vec!["new".into(), "old".into()];
        run(&mut state, Action::CliHistoryPrev);
        assert_eq!(state.cli_input, "new");
        run(&mut state, Action::CliHistoryPrev);
        run(&mut state, Action::CliHistoryPrev);
        assert_eq!(state.cli_input, "old", "stops at the oldest");
        run(&mut state, Action::CliHistoryNext);
        assert_eq!(state.cli_input, "new");
        run(&mut state, Action::CliHistoryNext);
        assert_eq!(state.cli_input, "");
    }

    #[test]
    fn paths_are_quoted_when_needed() {
        let mut state = AppState::new(PathBuf::from("/a b"), PathBuf::from("/c"));
        run(&mut state, Action::InsertLeftPathToCli);
        run(&mut state, Action::InsertRightPathToCli);
        let right = PathBuf::from("/c").to_string_lossy().into_owned();
        let left = PathBuf::from("/a b").to_string_lossy().into_owned();
        assert_eq!(state.cli_input, format!("\"{left}\" {right} "));
        run(&mut state, Action::ClearCli);
        assert!(state.cli_input.is_empty());
    }

    #[test]
    fn inactive_panel_toggles_the_other_side() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        run(&mut state, Action::ToggleInactivePanel);
        assert!(state.panels.left_visible && !state.panels.right_visible);
    }
}
