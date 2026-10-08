//! Screens list (F12): switch between panels, viewers, editors and terminals.

use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::{AppState, PopupType, Screen};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let screens = state.screens.len();
    let Some(PopupType::ScreensMenu { cursor_idx, .. }) = state.dialogs.top_mut() else {
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
        ListKey::Close => {
            // Back to the dialog that was open on this screen, if any.
            let suspended = take_suspended(state);
            state.dialogs.set(suspended);
        }
        ListKey::Activate(idx) => switch_to(state, idx),
        ListKey::Other if key.code == KeyCode::F(3) => view_terminal_output(state, cursor),
        ListKey::Other => return Err(()),
    }
    Ok(None)
}

/// Removes the menu, returning the dialog it was opened over.
fn take_suspended(state: &mut AppState) -> Option<PopupType> {
    match state.dialogs.pop() {
        Some(PopupType::ScreensMenu {
            suspended_popup, ..
        }) => suspended_popup.map(|p| *p),
        _ => None,
    }
}

/// Enter: show screen `idx`, parking the current screen's dialog.
fn switch_to(state: &mut AppState, idx: usize) {
    let suspended = take_suspended(state);
    if idx >= state.screens.len() {
        state.dialogs.clear();
        return;
    }
    if idx == state.active_screen_idx {
        state.dialogs.set(suspended);
        return;
    }
    let mut parked = crate::app::state::DialogStack::new();
    parked.set(suspended);
    state.screen_dialogs[state.active_screen_idx] = parked;
    state.active_screen_idx = idx;
    state.dialogs = std::mem::take(&mut state.screen_dialogs[idx]);
}

/// F3 on a terminal screen: open its output in the viewer.
fn view_terminal_output(state: &mut AppState, idx: usize) {
    let Some(Screen::Terminal(ts)) = state.screens.get(idx) else {
        return;
    };
    let mut viewer = crate::ui::viewer::ViewerState::from_text(
        std::path::PathBuf::from(format!("Terminal: {}", ts.command)),
        ts.output_lines.clone(),
    );
    viewer.raw = ts.output_lines.join("\n").into_bytes();
    state.push_screen(Screen::Viewer(viewer));
    state.dialogs.clear();
}
