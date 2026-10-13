use crate::app::context::AppContext;
use crate::app::list_nav::{ScrollKeys, scroll_key};
use crate::app::state::AppState;
use crate::app::state::popup::{About, Dialog, DialogScroll, KeyHandler};
use crate::keybindings::Action;
use crate::ui::scrollbar::ScrollTargetId;
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
    let Some(About { scroll_y }) = state.dialogs.top_mut_as::<About>() else {
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

impl Dialog for About {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn closes_on_outside_click(&self) -> bool {
        true
    }
    fn resizable(&self) -> bool {
        true
    }
    fn scroll_mut(&mut self, id: ScrollTargetId) -> Option<DialogScroll<'_>> {
        (id == ScrollTargetId::About).then_some(DialogScroll::Offset(&mut self.scroll_y))
    }
}
