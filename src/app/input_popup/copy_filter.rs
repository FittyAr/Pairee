//! File mask prompt opened over the Copy / Move dialog.

use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::state::popup::{CopyMoveFilterPrompt, Dialog, KeyHandler, KeyOutcome};
use crate::app::state::{AppState, CopyMovePromptState};
use crate::app::text_input::TextField;
use crossterm::event::KeyEvent;

pub fn handle(state: &mut AppState, key: KeyEvent, _context: &mut AppContext) -> KeyOutcome {
    let Some(prompt) = state.dialogs.top_mut_as::<CopyMoveFilterPrompt>() else {
        return Err(());
    };
    let submit = match field_key(&mut prompt.input, &key) {
        FieldKey::Submit => true,
        FieldKey::Cancel => false,
        FieldKey::Handled | FieldKey::Other => return Ok(None),
    };
    // Back to the transfer dialog underneath, keeping the mask on Enter.
    if let Some(prompt) = state.dialogs.pop_as::<CopyMoveFilterPrompt>()
        && submit
        && let Some(transfer) = state.dialogs.top_mut_as::<CopyMovePromptState>()
    {
        transfer.filter_mask = prompt.input.into_text();
    }
    Ok(None)
}

impl Dialog for CopyMoveFilterPrompt {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        Some(&mut self.input)
    }
}
