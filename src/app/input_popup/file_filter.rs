//! Panel filter (persistent mask) and quick filter (live, restored on Esc).

use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    match state.dialogs.top_mut() {
        Some(PopupType::FilePanelFilterPrompt { input }) => {
            match field_key(input, &key) {
                FieldKey::Cancel => state.dialogs.clear(),
                FieldKey::Submit => {
                    let mask = input.text().trim().to_string();
                    state.dialogs.clear();
                    state.get_active_panel_mut().filter_mask = (!mask.is_empty()).then_some(mask);
                    state.refresh_both_panels(context.config.settings.show_hidden);
                }
                FieldKey::Handled | FieldKey::Other => {}
            }
            Ok(None)
        }
        Some(PopupType::QuickFilterPrompt { input, .. }) => {
            let active = state.panels.active;
            match field_key(input, &key) {
                FieldKey::Submit => state.dialogs.clear(),
                FieldKey::Cancel => {
                    if let Some(PopupType::QuickFilterPrompt {
                        original_mask,
                        original_cursor,
                        ..
                    }) = state.dialogs.pop()
                    {
                        state.dialogs.clear();
                        state.update_panel_filter(active, original_mask);
                        state.panels.side_mut(active).cursor_index = original_cursor;
                    }
                }
                FieldKey::Handled => {
                    let mask = input.text().to_string();
                    state.update_panel_filter(active, Some(mask));
                }
                FieldKey::Other => {}
            }
            Ok(None)
        }
        _ => Err(()),
    }
}
