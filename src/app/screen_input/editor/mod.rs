//! Key and mouse handling for the built-in editor screen.
//!
//! Keys are tried in order: global panel actions (passed on), the
//! `[editor]` keymap commands (save, search, clipboard, quit, …), cursor
//! motions (with selection), then typing.

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
    // Screens list, help, palette... work over the editor too.
    if context.resolver.global_action(key).is_some() || state.active_editor_mut().is_none() {
        return Err(());
    }
    let height = editor_page_height();
    if let Some(command) = context.resolver.editor.dispatch(key) {
        let edit = commands::run(state, command, context, height);
        return finish(state, edit, height);
    }
    if context.resolver.editor.is_ongoing() {
        return Ok(());
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
    let edit = typing_key(state, &key, &options);
    finish(state, edit, height)
}

/// Explains a refused edit and keeps the cursor on screen.
fn finish(state: &mut AppState, edit: Edit, height: usize) -> Result<(), ()> {
    if let Edit::Locked = edit {
        state
            .dialogs
            .replace(PopupType::Info(t("editor_read_only_locked")));
    } else if let Some(ed) = state.active_editor_mut() {
        ed.ensure_cursor_visible(height);
    }
    Ok(())
}

/// Typing, Tab, Enter, Backspace and Delete.
fn typing_key(state: &mut AppState, key: &KeyEvent, options: &EditorOptions) -> Edit {
    let Some(ed) = state.active_editor_mut() else {
        return Edit::Ignored;
    };
    let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let done = match key.code {
        KeyCode::Char(c) if !is_ctrl => ed.insert_char(c),
        KeyCode::Tab => ed.insert_tab(options),
        KeyCode::Enter => ed.insert_newline(options.auto_indent),
        KeyCode::Backspace => ed.backspace(),
        KeyCode::Delete => ed.delete_forward(),
        _ => return Edit::Ignored,
    };
    edit_result(done, ed)
}
