use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::AppState;
use crate::app::state::popup::ContextMenu;
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

/// Context-menu entries and the action each one runs (matched by label).
const ITEM_ACTIONS: [(&str, Action); 7] = [
    ("View", Action::View),
    ("Edit", Action::Edit),
    ("Copy", Action::Copy),
    ("Move", Action::Move),
    ("Delete", Action::Delete),
    ("Compress", Action::CompressFiles),
    ("Extract", Action::ExtractArchive),
];

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(ContextMenu { items, cursor_idx }) = state.dialogs.top_mut_as::<ContextMenu>() else {
        return Err(());
    };
    match list_key(ListKeys::ARROWS, key.code, cursor_idx, items.len()) {
        ListKey::Moved => Ok(None),
        ListKey::Close => {
            state.dialogs.clear();
            Ok(None)
        }
        ListKey::Activate(idx) => {
            let Some(item) = items.get(idx) else {
                return Ok(None);
            };
            let action = ITEM_ACTIONS
                .iter()
                .find(|(label, _)| item.contains(label))
                .map(|(_, action)| *action);
            state.dialogs.clear();
            Ok(action)
        }
        ListKey::Other => Err(()),
    }
}

impl Dialog for ContextMenu {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn closes_on_outside_click(&self) -> bool {
        true
    }
}
