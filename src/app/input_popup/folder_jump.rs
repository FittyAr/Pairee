//! Keys of the folder jump list: typing filters, Up / Down move, Enter
//! goes to the folder, Esc closes.

use crate::app::context::AppContext;
use crate::app::folder_jump::{go, matches};
use crate::app::list_nav::{FilterKey, filter_list_key};
use crate::app::state::AppState;
use crate::app::state::popup::{Dialog, FolderJump, KeyHandler};
use crate::app::text_input::TextField;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(FolderJump {
        query,
        cursor_idx,
        items,
    }) = state.dialogs.top_mut_as::<FolderJump>()
    else {
        return Err(());
    };
    match filter_list_key(query, cursor_idx, items.len(), &key) {
        FilterKey::Moved => {}
        FilterKey::QueryChanged => {
            let query = query.text().to_string();
            let found = matches(state, &query);
            if let Some(jump) = state.dialogs.top_mut_as::<FolderJump>() {
                jump.items = found;
                jump.cursor_idx = 0;
            }
        }
        FilterKey::Activate(idx) => {
            if let Some(path) = items.get(idx).cloned() {
                go(state, path);
                state.refresh_both_panels(context.config.settings.show_hidden);
            }
        }
        FilterKey::Close => state.dialogs.clear(),
    }
    Ok(None)
}

impl Dialog for FolderJump {
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
