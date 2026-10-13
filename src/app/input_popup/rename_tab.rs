use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::state::AppState;
use crate::app::state::popup::RenameTabPrompt;
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::app::text_input::TextField;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

/// Tab title prompt: Enter applies (an empty title goes back to the folder
/// name), Esc cancels.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(RenameTabPrompt { input, .. }) = state.dialogs.top_mut_as::<RenameTabPrompt>() else {
        return Err(());
    };
    match field_key(input, &key) {
        FieldKey::Cancel => state.dialogs.clear(),
        FieldKey::Submit => {
            if let Some(RenameTabPrompt { tab, input }) = state.dialogs.pop_as::<RenameTabPrompt>()
            {
                state.dialogs.clear();
                state.rename_tab(tab, input.text());
            }
        }
        FieldKey::Handled | FieldKey::Other => {}
    }
    Ok(None)
}

impl Dialog for RenameTabPrompt {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        Some(&mut self.input)
    }
}
