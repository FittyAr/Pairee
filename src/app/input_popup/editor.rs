//! Popups of the built-in editor: search, "save as" and overwrite confirmation.

use crate::app::context::AppContext;
use crate::app::editor::open::{resolve_save_as_path, save_active_editor};
use crate::app::form::{FieldKey, confirm_answer, field_key};
use crate::app::screen_input::editor::editor_page_height;
use crate::app::state::popup::SearchKey;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    match state.dialogs.top_mut() {
        Some(PopupType::EditorSearchPrompt(search)) => {
            let request = search.handle_key(&key);
            let query = search.query.text().to_string();
            let case_sensitive = search.case_sensitive;
            match request {
                SearchKey::Stay => {}
                SearchKey::Close => state.dialogs.clear(),
                SearchKey::Find if query.is_empty() => state.dialogs.clear(),
                SearchKey::Find => find_next(state, &query, case_sensitive),
            }
            Ok(None)
        }
        Some(PopupType::EditorSaveAsPrompt { input }) => {
            match field_key(input, &key) {
                FieldKey::Cancel => state.dialogs.clear(),
                FieldKey::Submit => {
                    let input = input.text().to_string();
                    save_as(state, &input);
                }
                FieldKey::Handled | FieldKey::Other => {}
            }
            Ok(None)
        }
        Some(PopupType::EditorConfirmOverwrite { target, .. }) => {
            let target = target.clone();
            match confirm_answer(&key, true) {
                Some(true) => {
                    state.dialogs.clear();
                    save_active_editor(state, Some(target), true);
                }
                Some(false) => state.dialogs.clear(),
                None => {}
            }
            Ok(None)
        }
        _ => Err(()),
    }
}

/// Enter in the find dialog: jump to the next match (or report none).
fn find_next(state: &mut AppState, query: &str, case_sensitive: bool) {
    let height = editor_page_height();
    let Some(ed) = state.active_editor_mut() else {
        return state.dialogs.clear();
    };
    if !ed.find_next(query, case_sensitive, height) {
        state
            .dialogs
            .replace(PopupType::Error(t("editor_text_not_found")));
    }
}

/// Enter in "save as": save to the typed path (relative to the file's folder).
fn save_as(state: &mut AppState, input: &str) {
    let target = state
        .active_editor_mut()
        .and_then(|ed| resolve_save_as_path(&ed.path, input));
    state.dialogs.clear();
    if let Some(target) = target {
        save_active_editor(state, Some(target), false);
    }
}
