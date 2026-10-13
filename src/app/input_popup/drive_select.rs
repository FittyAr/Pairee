use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::popup::DriveSelect;
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::DriveSelect(DriveSelect {
        panel,
        drives,
        cursor_idx,
    })) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    match list_key(ListKeys::ARROWS, key.code, cursor_idx, drives.len()) {
        ListKey::Moved => {}
        ListKey::Close => state.dialogs.clear(),
        ListKey::Activate(idx) => {
            let panel = *panel;
            if let Some(drive) = drives.get(idx).map(std::path::PathBuf::from) {
                state.panels.side_mut(panel).open_path(drive);
                state.dialogs.clear();
                state.refresh_both_panels(context.config.settings.show_hidden);
            }
        }
        ListKey::Other => return Err(()),
    }
    Ok(None)
}

impl Dialog for DriveSelect {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn closes_on_outside_click(&self) -> bool {
        true
    }
}
