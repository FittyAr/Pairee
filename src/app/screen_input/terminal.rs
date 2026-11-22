//! Terminal screen keys: global panel actions (screens list, help...) pass
//! through, the `[terminal]` keymap section scrolls, stops or closes, and
//! every other key is typed into the running process.

use crate::app::context::AppContext;
use crate::app::state::{AppState, Screen};
use crate::keybindings::screens::TerminalAction;
use crate::terminal::keys::key_bytes;
use crossterm::event::KeyEvent;

/// Lines moved by the page keys.
const PAGE: usize = 20;

pub fn handle_terminal_screen(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<(), ()> {
    if context.resolver.global_action(key).is_some() {
        return Err(());
    }
    if let Some(command) = context.resolver.terminal.dispatch(key) {
        run(state, command);
        return Ok(());
    }
    if let Screen::Terminal(ts) = state.screens.active_mut()
        && let Some(bytes) = key_bytes(key)
        && ts.send_input(&bytes)
    {
        ts.scroll = 0;
    }
    Ok(())
}

/// Types a paste into the active terminal screen's process. `false` when
/// the active screen is not a running terminal.
pub fn paste_into_terminal(state: &mut AppState, text: &str) -> bool {
    match state.screens.active_mut() {
        Screen::Terminal(ts) => ts.send_input(text.as_bytes()),
        _ => false,
    }
}

fn run(state: &mut AppState, command: TerminalAction) {
    use TerminalAction as T;
    if command == T::Close {
        state.close_current_screen();
        return;
    }
    let Screen::Terminal(ts) = state.screens.active_mut() else {
        return;
    };
    match command {
        T::ScrollUp => ts.scroll_by(1, true),
        T::ScrollDown => ts.scroll_by(1, false),
        T::PageUp => ts.scroll_by(PAGE, true),
        T::PageDown => ts.scroll_by(PAGE, false),
        T::Top => ts.scroll_by(ts.output.len(), true),
        T::Bottom => ts.scroll = 0,
        T::Kill => ts.kill(),
        T::Close => {}
    }
}
