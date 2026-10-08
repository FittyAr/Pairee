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
    let Some(PopupType::ApplyCommandPrompt { input, .. }) = state.dialogs.top_mut() else {
        return Err(());
    };
    match field_key(input, &key) {
        FieldKey::Cancel => state.dialogs.clear(),
        FieldKey::Submit => {
            if let Some(PopupType::ApplyCommandPrompt { input, targets }) = state.dialogs.pop() {
                state.dialogs.clear();
                if !input.is_empty() {
                    crate::fs::transfer::submit_apply_command(state, input.into_text(), targets);
                }
            }
        }
        FieldKey::Handled | FieldKey::Other => {}
    }
    Ok(None)
}
