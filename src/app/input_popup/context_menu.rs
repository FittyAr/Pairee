use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::AppState;
use crate::app::state::popup::ContextMenu;
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

/// Context-menu entries (label keys) and the action each one runs; the
/// last one is only offered for archives.
pub const ITEM_ACTIONS: [(&str, Action); 7] = [
    ("ctx_menu_view", Action::View),
    ("ctx_menu_edit", Action::Edit),
    ("ctx_menu_copy", Action::Copy),
    ("ctx_menu_move", Action::Move),
    ("ctx_menu_delete", Action::Delete),
    ("ctx_menu_compress", Action::CompressFiles),
    ("ctx_menu_extract", Action::ExtractArchive),
];

/// The action of the entry labelled `item` (in the current language).
fn action_of(item: &str) -> Option<Action> {
    ITEM_ACTIONS
        .iter()
        .find(|(key, _)| t(key) == item)
        .map(|(_, action)| *action)
}

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
            let action = action_of(item);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_runs_its_action_in_any_language() {
        for (key, action) in ITEM_ACTIONS {
            assert_eq!(action_of(&t(key)), Some(action), "{key}");
        }
        assert_eq!(action_of("Ver"), None, "labels are matched whole");
    }
}
