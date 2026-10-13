//! Screens list (F12): switch between panels, viewers, editors and terminals.

use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::popup::ScreensMenu;
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::app::state::{AppState, PopupType, Screen};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let screens = state.screens.len();
    let Some(PopupType::ScreensMenu(ScreensMenu { cursor_idx, .. })) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    let code = if key.code == KeyCode::F(12) {
        KeyCode::Esc
    } else {
        key.code
    };
    let action = list_key(ListKeys::ARROWS, code, cursor_idx, screens);
    let cursor = *cursor_idx;
    match action {
        ListKey::Moved => {}
        // Back to the dialogs open on this screen, if any.
        ListKey::Close => drop(state.dialogs.pop_as::<ScreensMenu>()),
        ListKey::Activate(idx) => switch_to(state, idx),
        ListKey::Other if key.code == KeyCode::F(3) => view_terminal_output(state, cursor),
        ListKey::Other if key.code == KeyCode::Delete => close_at(state, cursor),
        ListKey::Other => return Err(()),
    }
    Ok(None)
}

/// Enter: show screen `idx`, parking the current screen's dialogs.
fn switch_to(state: &mut AppState, idx: usize) {
    state.dialogs.pop_as::<ScreensMenu>();
    if idx >= state.screens.len() {
        state.dialogs.clear();
        return;
    }
    state.switch_screen(idx);
}

/// Delete: closes screen `idx` (never the panels), stopping its process.
fn close_at(state: &mut AppState, idx: usize) {
    let Some(id) = state.screens.id_at(idx).filter(|_| idx > 0) else {
        return;
    };
    state.close_screen(id);
    let last = state.screens.len() - 1;
    if let Some(PopupType::ScreensMenu(ScreensMenu { cursor_idx, .. })) = state.dialogs.top_mut() {
        *cursor_idx = (*cursor_idx).min(last);
    }
}

/// F3 on a terminal screen: open its output in the viewer.
fn view_terminal_output(state: &mut AppState, idx: usize) {
    let Some(Screen::Terminal(ts)) = state.screens.get(idx) else {
        return;
    };
    let title = t("terminal_view_output_title").replacen("{}", &ts.command, 1);
    let viewer =
        crate::ui::viewer::ViewerState::from_text(std::path::PathBuf::from(title), ts.output.all());
    state.push_screen(Screen::Viewer(viewer));
    state.dialogs.clear();
}

impl Dialog for ScreensMenu {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn closes_on_outside_click(&self) -> bool {
        true
    }
}
