//! Editor screen commands, bound in the `[editor]` keymap section: save,
//! save as, reload, search and replace, viewer, block mode, undo / redo,
//! clipboard and quit.

use super::{Edit, clipboard, edit_result, search};
use crate::app::context::AppContext;
use crate::app::editor::open::{reload_active_editor, save_active_editor};
use crate::app::state::popup::ConfirmDiscardEditorChanges;
use crate::app::state::popup::ConfirmReload;
use crate::app::state::popup::EditorSaveAsPrompt;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::screens::EditorAction;

/// Runs `command` on the active editor.
pub(super) fn run(
    state: &mut AppState,
    command: EditorAction,
    context: &AppContext,
    height: usize,
) -> Edit {
    use EditorAction as E;
    match command {
        E::Copy | E::Cut | E::Paste | E::SelectAll => return clipboard::run(state, command),
        E::Search | E::Replace => {
            search::open_dialog(state, command == E::Replace);
            return Edit::Done;
        }
        E::SearchNext | E::SearchPrev => {
            search::repeat(state, command == E::SearchPrev, height);
            return Edit::Done;
        }
        _ => {}
    }
    let settings = &context.config.settings;
    let Some(ed) = state.active_editor_mut() else {
        return Edit::Ignored;
    };
    match command {
        E::Undo => return edit_result(ed.undo(), ed),
        E::Redo => return edit_result(ed.redo(), ed),
        E::SaveAs => {
            let name = ed
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            state
                .dialogs
                .replace(EditorSaveAsPrompt { input: name.into() });
        }
        E::Save => {
            save_active_editor(state, None, false);
        }
        E::ToggleBlockMode => ed.block_mode = !ed.block_mode,
        E::Reload => {
            if ed.is_dirty() && settings.confirmations.confirm_reload_edited_file {
                state.dialogs.replace(ConfirmReload);
            } else {
                reload_active_editor(state);
            }
        }
        E::OpenViewer => {
            let path = ed.path.clone();
            state.open_viewer(path, settings, true);
        }
        E::Discard => state
            .dialogs
            .replace(PopupType::ConfirmDiscardEditorChanges(
                ConfirmDiscardEditorChanges,
            )),
        E::Quit if ed.has_selection() || ed.show_matches => {
            ed.selection = None;
            ed.show_matches = false;
        }
        E::Quit if ed.is_dirty() => state
            .dialogs
            .replace(PopupType::ConfirmDiscardEditorChanges(
                ConfirmDiscardEditorChanges,
            )),
        E::Quit => state.close_current_screen(),
        E::Copy
        | E::Cut
        | E::Paste
        | E::SelectAll
        | E::Search
        | E::Replace
        | E::SearchNext
        | E::SearchPrev => {}
    }
    Edit::Done
}
