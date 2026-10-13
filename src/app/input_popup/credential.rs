//! Keys of the credential prompt: Enter sends the secret to the job that
//! asked, Esc cancels it.

use crate::app::context::AppContext;
use crate::app::credential_requests::finish_prompt;
use crate::app::form::{FieldKey, field_key};
use crate::app::state::AppState;
use crate::app::state::popup::{CredentialPrompt, Dialog, KeyHandler};
use crate::app::text_input::TextField;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(prompt) = state.dialogs.top_mut_as::<CredentialPrompt>() else {
        return Err(());
    };
    match field_key(&mut prompt.input, &key) {
        FieldKey::Submit => finish_prompt(state, true),
        FieldKey::Cancel => finish_prompt(state, false),
        FieldKey::Handled | FieldKey::Other => {}
    }
    Ok(None)
}

impl Dialog for CredentialPrompt {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        Some(&mut self.input)
    }
}
