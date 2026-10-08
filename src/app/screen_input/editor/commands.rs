//! Editor screen commands: save, save as, reload, search, viewer, quit.

use crate::app::context::AppContext;
use crate::app::editor::open::{reload_active_editor, save_active_editor};
use crate::app::state::{AppState, PopupType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Runs the command bound to `key`, if any.
pub(super) fn handle(state: &mut AppState, key: &KeyEvent, context: &AppContext, height: usize) {
    let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let settings = &context.config.settings;
    let Some(ed) = state.active_editor_mut() else {
        return;
    };
    match key.code {
        KeyCode::F(2) if is_shift => {
            let name = ed
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            state
                .dialogs
                .replace(PopupType::EditorSaveAsPrompt { input: name.into() });
        }
        KeyCode::F(2) => {
            save_active_editor(state, None, false);
        }
        KeyCode::Char('b') if is_ctrl => ed.block_mode = !ed.block_mode,
        KeyCode::Char('s') if is_ctrl => {
            save_active_editor(state, None, false);
        }
        KeyCode::Char('r') | KeyCode::Char('d') if is_ctrl => {
            if ed.is_dirty() && settings.confirmations.confirm_reload_edited_file {
                state.dialogs.replace(PopupType::ConfirmReload);
            } else {
                reload_active_editor(state);
            }
        }
        KeyCode::F(7) if is_shift => {
            ed.repeat_search(height);
        }
        KeyCode::F(3) => {
            ed.repeat_search(height);
        }
        KeyCode::F(7) | KeyCode::Char('f') if is_ctrl || key.code == KeyCode::F(7) => {
            state
                .dialogs
                .replace(PopupType::EditorSearchPrompt(Default::default()));
        }
        KeyCode::F(4) => {
            let path = ed.path.clone();
            state.open_viewer(path, settings, true);
        }
        KeyCode::F(8) => {
            state
                .dialogs
                .replace(PopupType::ConfirmDiscardEditorChanges);
        }
        KeyCode::Esc if ed.has_selection() => ed.selection = None,
        KeyCode::Esc | KeyCode::F(10) => {
            if ed.is_dirty() {
                state
                    .dialogs
                    .replace(PopupType::ConfirmDiscardEditorChanges);
            } else {
                state.close_current_screen();
            }
        }
        _ => {}
    }
}
