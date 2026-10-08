//! Clipboard keys and bracketed paste in the editor: `Ctrl+C`/`Ctrl+Insert`
//! copy, `Ctrl+X`/`Shift+Delete` cut, `Ctrl+V`/`Shift+Insert` paste and
//! `Ctrl+A` select all.

use super::{Edit, edit_result};
use crate::app::state::{AppState, PopupType, Screen};
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipOp {
    Copy,
    Cut,
    Paste,
    SelectAll,
}

fn op_for(key: &KeyEvent) -> Option<ClipOp> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    match key.code {
        KeyCode::Insert if ctrl => Some(ClipOp::Copy),
        KeyCode::Insert if shift => Some(ClipOp::Paste),
        KeyCode::Delete if shift => Some(ClipOp::Cut),
        KeyCode::Char(c) if ctrl => match c.to_ascii_lowercase() {
            'c' => Some(ClipOp::Copy),
            'x' => Some(ClipOp::Cut),
            'v' => Some(ClipOp::Paste),
            'a' => Some(ClipOp::SelectAll),
            _ => None,
        },
        _ => None,
    }
}

/// Handles a clipboard key; `None` for any other key.
pub(super) fn handle_key(state: &mut AppState, key: &KeyEvent) -> Option<Edit> {
    let op = op_for(key)?;
    let Some(Screen::Editor(ed)) = state.screens.get_mut(state.active_screen_idx) else {
        return None;
    };
    let clipboard = &mut state.editor_clipboard;
    let edit = match op {
        ClipOp::Copy => {
            if let Some(text) = ed.selected_text() {
                clipboard.copy(text);
            }
            Edit::Done
        }
        ClipOp::Cut => match ed.cut_selection() {
            Some(text) => {
                clipboard.copy(text);
                Edit::Done
            }
            None => edit_result(false, ed),
        },
        ClipOp::Paste => {
            let done = clipboard.paste().is_some_and(|text| ed.insert_text(&text));
            edit_result(done, ed)
        }
        ClipOp::SelectAll => {
            ed.select_all();
            Edit::Done
        }
    };
    Some(edit)
}

/// Bracketed paste on the editor screen: the whole text (all lines) is
/// inserted as one undo step. Returns `false` when no editor is active.
pub fn paste_into_editor(state: &mut AppState, text: &str) -> bool {
    let Some(ed) = state.active_editor_mut() else {
        return false;
    };
    let done = ed.insert_text(text);
    ed.ensure_cursor_visible(super::editor_page_height());
    if !done && ed.locked {
        state
            .dialogs
            .replace(PopupType::Info(t("editor_read_only_locked")));
    }
    true
}
