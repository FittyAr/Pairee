use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::state::popup::SelectGroupPrompt;
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::app::state::{AppState, SelectMode};
use crate::app::text_input::TextField;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(SelectGroupPrompt { query, .. }) = state.dialogs.top_mut_as::<SelectGroupPrompt>()
    else {
        return Err(());
    };
    match field_key(query, &key) {
        FieldKey::Cancel => state.dialogs.clear(),
        FieldKey::Submit => {
            if let Some(SelectGroupPrompt { mode, query }) =
                state.dialogs.pop_as::<SelectGroupPrompt>()
            {
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

impl Dialog for SelectGroupPrompt {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        Some(&mut self.query)
    }
}
