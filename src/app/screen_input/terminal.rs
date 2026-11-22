//! Terminal screen keys: global panel actions (screens list, help...) pass
//! through, the `[terminal]` keymap section scrolls, stops, closes or goes
//! back to the panels, and every other key is typed into the running
//! process. Once it ended, Enter or Esc closes the screen.

use crate::app::context::AppContext;
use crate::app::state::{AppState, Screen};
use crate::keybindings::screens::TerminalAction;
use crate::terminal::keys::key_bytes;
use crate::terminal::vt::cursor_keys;
use crossterm::event::{KeyCode, KeyEvent};

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
    let Screen::Terminal(ts) = state.screens.active_mut() else {
        return Ok(());
    };
    if !ts.is_running() && matches!(key.code, KeyCode::Enter | KeyCode::Esc) {
        state.close_current_screen();
        return Ok(());
    }
    let application = ts.vt.as_ref().is_some_and(|vt| vt.application_cursor());
    if let Some(bytes) = key_bytes(key).map(|b| cursor_keys(b, application))
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
        Screen::Terminal(ts) => {
            // Programs that asked for bracketed paste take it as text,
            // not as typed keys (an editor does not auto-indent it).
            if ts.vt.as_ref().is_some_and(|vt| vt.bracketed_paste()) {
                let wrapped = format!("\x1b[200~{text}\x1b[201~");
                ts.send_input(wrapped.as_bytes())
            } else {
                ts.send_input(text.as_bytes())
            }
        }
        _ => false,
    }
}

fn run(state: &mut AppState, command: TerminalAction) {
    use TerminalAction as T;
    if command == T::Close {
        state.close_current_screen();
        return;
    }
    if command == T::Panels {
        // The panels are always the first screen.
        state.switch_screen(0);
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
        T::Top => ts.scroll_by(ts.scroll_limit(), true),
        T::Bottom => ts.scroll = 0,
        T::Kill => ts.kill(),
        T::Close | T::Panels => {}
    }
}
