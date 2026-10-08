use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

/// Tab title prompt: Enter applies (an empty title goes back to the folder
/// name), Esc cancels.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::RenameTabPrompt { input, .. }) = state.dialogs.top_mut() else {
        return Err(());
    };
    match field_key(input, &key) {
        FieldKey::Cancel => state.dialogs.clear(),
        FieldKey::Submit => {
            if let Some(PopupType::RenameTabPrompt { tab, input }) = state.dialogs.pop() {
                state.dialogs.clear();
                state.rename_tab(tab, input.text());
            }
        }
        FieldKey::Handled | FieldKey::Other => {}
    }
    Ok(None)
}
