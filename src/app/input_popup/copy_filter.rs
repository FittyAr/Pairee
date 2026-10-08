//! File mask prompt opened from the Copy / Move dialog.

use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::CopyMoveFilterPrompt { input, .. }) = state.dialogs.top_mut() else {
        return Err(());
    };
    let submit = match field_key(input, &key) {
        FieldKey::Submit => true,
        FieldKey::Cancel => false,
        FieldKey::Handled | FieldKey::Other => return Ok(None),
    };
    // Back to the transfer dialog, keeping the mask on Enter.
    if let Some(PopupType::CopyMoveFilterPrompt {
        input,
        mut previous,
    }) = state.dialogs.pop()
    {
        if let (true, PopupType::TransferPrompt(prompt)) = (submit, previous.as_mut()) {
            prompt.filter_mask = input.into_text();
        }
        state.dialogs.replace(*previous);
    }
    Ok(None)
}
