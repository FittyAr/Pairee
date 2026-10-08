use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

/// Lines scrolled by PgUp / PgDn in the About box.
const PAGE: usize = 15;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::About { scroll_y }) = state.dialogs.top_mut() else {
        return Err(());
    };
    match key.code {
        KeyCode::Esc | KeyCode::Enter => state.dialogs.clear(),
        KeyCode::Up | KeyCode::Char('k' | 'K') => *scroll_y = scroll_y.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j' | 'J') => *scroll_y = scroll_y.saturating_add(1),
        KeyCode::PageUp => *scroll_y = scroll_y.saturating_sub(PAGE),
        KeyCode::PageDown => *scroll_y = scroll_y.saturating_add(PAGE),
        _ => return Err(()),
    }
    Ok(None)
}
