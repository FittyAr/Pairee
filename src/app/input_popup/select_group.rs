use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::state::{AppState, PopupType, SelectMode};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::SelectGroupPrompt { query, .. }) = state.dialogs.top_mut() else {
        return Err(());
    };
    match field_key(query, &key) {
        FieldKey::Cancel => state.dialogs.clear(),
        FieldKey::Submit => {
            if let Some(PopupType::SelectGroupPrompt { mode, query }) = state.dialogs.pop() {
                state.dialogs.clear();
                let panel = state.get_active_panel_mut();
                match mode {
                    SelectMode::Add => panel.select_group(query.text()),
                    SelectMode::Remove => panel.unselect_group(query.text()),
                }
            }
        }
        FieldKey::Handled | FieldKey::Other => {}
    }
    Ok(None)
}
