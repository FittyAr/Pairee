use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::state::AppState;
use crate::app::state::popup::ApplyCommandPrompt;
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::app::text_input::TextField;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(ApplyCommandPrompt { input, .. }) = state.dialogs.top_mut_as::<ApplyCommandPrompt>()
    else {
        return Err(());
    };
    match field_key(input, &key) {
        FieldKey::Cancel => state.dialogs.clear(),
        FieldKey::Submit => {
            if let Some(ApplyCommandPrompt { input, targets }) =
                state.dialogs.pop_as::<ApplyCommandPrompt>()
            {
                state.dialogs.clear();
                if !input.is_empty() {
                    crate::app::transfer_submit::submit_apply_command(
                        state,
                        input.into_text(),
                        targets,
                    );
                }
            }
        }
        FieldKey::Handled | FieldKey::Other => {}
    }
    Ok(None)
}

impl Dialog for ApplyCommandPrompt {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        Some(&mut self.input)
    }
}
