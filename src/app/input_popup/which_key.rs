//! Input handling for the which-key overlay.

use crate::app::actions::which_key::filter_items;
use crate::app::context::AppContext;
use crate::app::list_nav::{FilterKey, filter_list_key};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::WhichKey {
        query,
        cursor_idx,
        items,
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    let visible = filter_items(query.text(), items);
    match filter_list_key(query, cursor_idx, visible.len(), &key) {
        FilterKey::Moved | FilterKey::QueryChanged => Ok(None),
        FilterKey::Activate(idx) => {
            let action = visible.get(idx).map(|(_, _, action)| *action);
            state.dialogs.clear();
            Ok(action)
        }
        FilterKey::Close => {
            state.dialogs.clear();
            Ok(None)
        }
    }
}
