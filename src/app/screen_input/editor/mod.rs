//! Key and mouse handling for the built-in editor screen.
//!
//! Keys are tried in order: cursor motions (with selection), text edits
//! (typing, clipboard), then screen commands (save, search, quit, …).

mod clipboard;
mod commands;
mod motion;
mod mouse;
#[cfg(test)]
mod tests;

pub use clipboard::paste_into_editor;
pub use mouse::handle_editor_mouse;

use crate::app::context::AppContext;
use crate::app::editor::{EditorOptions, EditorState};
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Rows available for text in the editor screen (terminal height minus
/// frame, status line and menu/key bars).
pub fn editor_page_height() -> usize {
    let term_height = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
    (term_height * 90 / 100).saturating_sub(3).max(1) as usize
}

/// Outcome of a key that edits text.
enum Edit {
    Done,
    /// Refused because the buffer is locked (read-only file).
    Locked,
    Ignored,
}

fn edit_result(done: bool, ed: &EditorState) -> Edit {
    match (done, ed.locked) {
        (true, _) => Edit::Done,
        (false, true) => Edit::Locked,
        (false, false) => Edit::Ignored,
    }
}

pub fn handle_editor_screen(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<(), ()> {
    let height = editor_page_height();
    let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    // Some global keys should still pass through like F12, Ctrl+Tab
    if key.code == KeyCode::F(12) || (key.code == KeyCode::Tab && is_ctrl) {
        return Err(()); // pass to global resolver
    }

    let options = EditorOptions::from(&context.config.settings);
    let Some(ed) = state.active_editor_mut() else {
        return Err(());
    };
    if let Some((motion, extend)) = motion::from_key(&key, height, ed.block_mode) {
        ed.move_cursor(motion, extend);
        ed.ensure_cursor_visible(height);
        return Ok(());
    }

    let edit = match clipboard::handle_key(state, &key) {
        Some(edit) => edit,
        None => typing_key(state, &key, &options),
    };
    match edit {
        Edit::Done => {}
        Edit::Locked => {
            state
                .dialogs
                .replace(PopupType::Info(t("editor_read_only_locked")));
            return Ok(());
        }
        Edit::Ignored => commands::handle(state, &key, context, height),
    }
    if let Some(ed) = state.active_editor_mut() {
        ed.ensure_cursor_visible(height);
    }
    Ok(())
}

/// Typing, Tab, Enter, Backspace/Delete and undo/redo.
fn typing_key(state: &mut AppState, key: &KeyEvent, options: &EditorOptions) -> Edit {
    let Some(ed) = state.active_editor_mut() else {
        return Edit::Ignored;
    };
    let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let done = match key.code {
        KeyCode::Char(c) if !is_ctrl => ed.insert_char(c),
        KeyCode::Tab => ed.insert_tab(options),
        KeyCode::Enter => ed.insert_newline(options.auto_indent),
        KeyCode::Backspace => ed.backspace(),
        KeyCode::Delete => ed.delete_forward(),
        KeyCode::Char('z') if is_ctrl && is_shift => ed.redo(),
        KeyCode::Char('Z') if is_ctrl => ed.redo(),
        KeyCode::Char('z') if is_ctrl => ed.undo(),
        KeyCode::Char('y') if is_ctrl => ed.redo(),
        _ => return Edit::Ignored,
    };
    edit_result(done, ed)
}
