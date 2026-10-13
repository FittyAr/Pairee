//! Input handling for the command palette popup.

use crate::app::actions::command_palette::filter_items;
use crate::app::context::AppContext;
use crate::app::list_nav::{FilterKey, filter_list_key};
use crate::app::state::popup::CommandPalette;
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::app::state::{AppState, PopupType};
use crate::app::text_input::TextField;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::CommandPalette(CommandPalette {
        query,
        cursor_idx,
        items,
    })) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    match filter_list_key(query, cursor_idx, items.len(), &key) {
        FilterKey::Moved => Ok(None),
        FilterKey::QueryChanged => {
            *items = filter_items(query.text());
            Ok(None)
        }
        FilterKey::Activate(idx) => {
            let action = items.get(idx).map(|(_, action)| *action);
            state.dialogs.clear();
            Ok(action)
        }
        FilterKey::Close => {
            state.dialogs.clear();
            Ok(None)
        }
    }
}

impl Dialog for CommandPalette {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn closes_on_outside_click(&self) -> bool {
        true
    }
    fn resizable(&self) -> bool {
        true
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        Some(&mut self.query)
    }
}
