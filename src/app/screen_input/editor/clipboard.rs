//! Clipboard commands and bracketed paste in the editor (the keys are in
//! the `[editor]` keymap section: `Ctrl+C` / `Ctrl+Insert` copy, …).

use super::{Edit, edit_result};
use crate::app::state::{AppState, PopupType, Screen};
use crate::config::localization::t;
use crate::keybindings::screens::EditorAction;

/// Runs a clipboard command (`copy`, `cut`, `paste`, `select_all`).
pub(super) fn run(state: &mut AppState, command: EditorAction) -> Edit {
    let Some(Screen::Editor(ed)) = state.screens.get_mut(state.active_screen_idx) else {
        return Edit::Ignored;
    };
    let clipboard = &mut state.editor_clipboard;
    match command {
        EditorAction::Copy => {
            if let Some(text) = ed.selected_text() {
                clipboard.copy(text);
            }
            Edit::Done
        }
        EditorAction::Cut => match ed.cut_selection() {
            Some(text) => {
                clipboard.copy(text);
                Edit::Done
            }
            None => edit_result(false, ed),
        },
        EditorAction::Paste => {
            let done = clipboard.paste().is_some_and(|text| ed.insert_text(&text));
            edit_result(done, ed)
        }
        EditorAction::SelectAll => {
            ed.select_all();
            Edit::Done
        }
        _ => Edit::Ignored,
    }
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
