//! Popups of the built-in editor: search, "save as" and overwrite confirmation.

use crate::app::context::AppContext;
use crate::app::editor::open::{resolve_save_as_path, save_active_editor};
use crate::app::screen_input::editor::editor_page_height;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    match state.dialogs.top().cloned() {
        Some(PopupType::EditorSearchPrompt {
            query,
            case_sensitive,
            cursor_idx,
        }) => {
            handle_search(state, key, query, case_sensitive, cursor_idx);
            Ok(None)
        }
        Some(PopupType::EditorSaveAsPrompt { input }) => {
            handle_save_as(state, key, input);
            Ok(None)
        }
        Some(PopupType::EditorConfirmOverwrite { target, .. }) => {
            match key.code {
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    state.dialogs.clear();
                    save_active_editor(state, Some(target), true);
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => state.dialogs.clear(),
                _ => {}
            }
            Ok(None)
        }
        _ => Err(()),
    }
}

fn handle_search(
    state: &mut AppState,
    key: KeyEvent,
    mut query: String,
    mut case_sensitive: bool,
    mut cursor_idx: usize,
) {
    let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Tab | KeyCode::Down => cursor_idx = (cursor_idx + 1) % 4,
        KeyCode::Up => cursor_idx = (cursor_idx + 3) % 4,
        KeyCode::Left | KeyCode::Right if cursor_idx >= 2 => cursor_idx = 5 - cursor_idx,
        KeyCode::Char(c) if cursor_idx == 0 && !is_ctrl => query.push(c),
        KeyCode::Char(' ') if cursor_idx == 1 => case_sensitive = !case_sensitive,
        KeyCode::Backspace if cursor_idx == 0 => {
            query.pop();
        }
        KeyCode::Esc => return state.dialogs.clear(),
        KeyCode::Enter if cursor_idx == 3 || query.is_empty() => return state.dialogs.clear(),
        KeyCode::Enter => {
            let height = editor_page_height();
            let Some(ed) = state.active_editor_mut() else {
                return state.dialogs.clear();
            };
            if !ed.find_next(&query, case_sensitive, height) {
                state
                    .dialogs
                    .replace(PopupType::Error(t("editor_text_not_found")));
                return;
            }
        }
        _ => {}
    }
    state.dialogs.replace(PopupType::EditorSearchPrompt {
        query,
        case_sensitive,
        cursor_idx,
    });
}

fn handle_save_as(state: &mut AppState, key: KeyEvent, mut input: String) {
    match key.code {
        KeyCode::Char(c) => input.push(c),
        KeyCode::Backspace => {
            input.pop();
        }
        KeyCode::Esc => return state.dialogs.clear(),
        KeyCode::Enter => {
            let target = state
                .active_editor_mut()
                .and_then(|ed| resolve_save_as_path(&ed.path, &input));
            state.dialogs.clear();
            if let Some(target) = target {
                save_active_editor(state, Some(target), false);
            }
            return;
        }
        _ => {}
    }
    state
        .dialogs
        .replace(PopupType::EditorSaveAsPrompt { input });
}
