//! Folder shortcuts dialog: assigns the active folder to Ctrl+Alt+1…9 slots.

use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::{AppState, PopupType};
use crate::config::bookmarks::{self, SHORTCUT_SLOTS};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

/// Number of slots shown in the dialog (1–9).
pub const SLOT_COUNT: usize = *SHORTCUT_SLOTS.end() as usize;

/// Slot number (1-based) for a cursor row.
pub fn slot_for_row(row: usize) -> u8 {
    (row + 1) as u8
}

/// Enter jumps, Ins (or the slot's digit) assigns the active folder, Del clears.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::FolderShortcuts { cursor_idx }) = state.dialogs.top_mut() else {
        return Err(());
    };
    let action = list_key(ListKeys::ARROWS, key.code, cursor_idx, SLOT_COUNT);
    let cursor_idx = *cursor_idx;
    let slot = slot_for_row(cursor_idx);
    match action {
        ListKey::Moved => return Ok(None),
        ListKey::Close => {
            state.dialogs.clear();
            return Ok(None);
        }
        ListKey::Activate(_) => {
            if let Some(target) = state.folder_shortcuts.get(&slot).cloned() {
                state.dialogs.clear();
                state.jump_active_panel_to(target, context.config.settings.show_hidden);
            }
            return Ok(None);
        }
        ListKey::Other => {}
    }
    match key.code {
        KeyCode::Insert | KeyCode::Char(' ') => assign(state, slot, cursor_idx),
        KeyCode::Char(c @ '1'..='9') => {
            let row = c as usize - '1' as usize;
            assign(state, slot_for_row(row), row);
        }
        KeyCode::Delete | KeyCode::Backspace => {
            if state.folder_shortcuts.remove(&slot).is_some() {
                persist(state);
            }
        }
        _ => return Err(()),
    }
    Ok(None)
}

fn assign(state: &mut AppState, slot: u8, row: usize) {
    let path = state.get_active_panel().current_path.clone();
    state.folder_shortcuts.insert(slot, path);
    state
        .dialogs
        .replace(PopupType::FolderShortcuts { cursor_idx: row });
    persist(state);
}

fn persist(state: &mut AppState) {
    if let Err(e) = bookmarks::save_shortcuts(&state.folder_shortcuts) {
        state.dialogs.push(PopupType::Error(
            t("error_save_bookmarks").replace("{}", &e.to_string()),
        ));
    }
}
