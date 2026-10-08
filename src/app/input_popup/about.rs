use crate::app::context::AppContext;
use crate::app::list_nav::{ScrollKeys, scroll_key};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

/// About box scrolling: arrows, k/j and 15-line pages.
pub const TEXT_SCROLL: ScrollKeys = ScrollKeys {
    vim: true,
    home_end: false,
    page: 15,
};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::About { scroll_y }) = state.dialogs.top_mut() else {
        return Err(());
    };
    if scroll_key(TEXT_SCROLL, key.code, scroll_y, usize::MAX) {
        return Ok(None);
    }
    match key.code {
        KeyCode::Esc | KeyCode::Enter => state.dialogs.clear(),
        _ => return Err(()),
    }
    Ok(None)
}
