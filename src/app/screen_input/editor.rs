//! Key handling for the built-in editor screen.

use crate::app::context::AppContext;
use crate::app::editor::open::{reload_active_editor, save_active_editor};
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
    let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);

    // Some global keys should still pass through like F12, Ctrl+Tab
    if key.code == KeyCode::F(12) || (key.code == KeyCode::Tab && is_ctrl) {
        return Err(()); // pass to global resolver
    }

    let settings = &context.config.settings;
    let options = EditorOptions::from(settings);
    let Some(ed) = state.active_editor_mut() else {
        return Err(());
    };

    let edit = match key.code {
        KeyCode::Char(c) if !is_ctrl => edit_result(ed.insert_char(c), ed),
        KeyCode::Tab => edit_result(ed.insert_tab(&options), ed),
        KeyCode::Enter => edit_result(ed.insert_newline(options.auto_indent), ed),
        KeyCode::Backspace => edit_result(ed.backspace(), ed),
        KeyCode::Delete => edit_result(ed.delete_forward(), ed),
        KeyCode::Char('z') if is_ctrl && is_shift => edit_result(ed.redo(), ed),
        KeyCode::Char('Z') if is_ctrl => edit_result(ed.redo(), ed),
        KeyCode::Char('z') if is_ctrl => edit_result(ed.undo(), ed),
        KeyCode::Char('y') if is_ctrl => edit_result(ed.redo(), ed),
        _ => Edit::Ignored,
    };
    match edit {
        Edit::Done => {
            ed.ensure_cursor_visible(height);
            return Ok(());
        }
        Edit::Locked => {
            state
                .dialogs
                .replace(PopupType::Info(t("editor_read_only_locked")));
            return Ok(());
        }
        Edit::Ignored => {}
    }

    match key.code {
        KeyCode::Up => ed.move_up(1),
        KeyCode::Down => ed.move_down(1),
        KeyCode::PageUp => ed.move_up(height),
        KeyCode::PageDown => ed.move_down(height),
        KeyCode::Left => ed.move_left(),
        KeyCode::Right => ed.move_right(),
        KeyCode::Home if is_ctrl => ed.move_doc_start(),
        KeyCode::End if is_ctrl => ed.move_doc_end(),
        KeyCode::Home => ed.move_line_start(),
        KeyCode::End => ed.move_line_end(),
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
    if let Some(ed) = state.active_editor_mut() {
        ed.ensure_cursor_visible(height);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Screen;
    use crate::config::AppConfig;
    use std::path::PathBuf;

    fn setup() -> (AppState, AppContext, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("doc.txt");
        std::fs::write(&path, "").unwrap();
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        let context = AppContext::new(AppConfig::default());
        crate::app::editor::open::open_in_editor(&mut state, path, &context.config.settings);
        (state, context, dir)
    }

    fn press_mod(state: &mut AppState, context: &mut AppContext, code: KeyCode, m: KeyModifiers) {
        handle_editor_screen(state, KeyEvent::new(code, m), context).expect("editor handles key");
    }

    fn press(state: &mut AppState, context: &mut AppContext, code: KeyCode) {
        press_mod(state, context, code, KeyModifiers::NONE);
    }

    fn editor(state: &AppState) -> &EditorState {
        match state.screens.get(state.active_screen_idx) {
            Some(Screen::Editor(ed)) => ed,
            _ => panic!("expected editor screen"),
        }
    }

    #[test]
    fn typing_and_editing_non_ascii_text() {
        let (mut state, mut context, _dir) = setup();
        for c in "ñandú 👍🏽e\u{301}".chars() {
            press(&mut state, &mut context, KeyCode::Char(c));
        }
        assert_eq!(editor(&state).lines[0], "ñandú 👍🏽e\u{301}");

        press(&mut state, &mut context, KeyCode::Backspace); // é (combined)
        press(&mut state, &mut context, KeyCode::Left); // before 👍🏽
        press(&mut state, &mut context, KeyCode::Delete); // 👍🏽
        assert_eq!(editor(&state).lines[0], "ñandú ");

        press(&mut state, &mut context, KeyCode::Left); // before ' '
        press(&mut state, &mut context, KeyCode::Left); // before 'ú'
        press(&mut state, &mut context, KeyCode::Char('X'));
        press(&mut state, &mut context, KeyCode::Enter);
        let ed = editor(&state);
        assert_eq!(ed.lines, vec!["ñandX".to_string(), "ú ".to_string()]);

        press(&mut state, &mut context, KeyCode::Right); // after 'ú'
        press(&mut state, &mut context, KeyCode::Up); // byte col 2 snaps inside "ñandX"
        let ed = editor(&state);
        assert_eq!(ed.cursor_y, 0);
        assert!(ed.lines[0].is_char_boundary(ed.cursor_x));
        press(&mut state, &mut context, KeyCode::Backspace);
        assert_eq!(editor(&state).lines[0], "andX");
    }

    #[test]
    fn ctrl_z_and_ctrl_y_undo_and_redo() {
        let (mut state, mut context, _dir) = setup();
        for c in "hi".chars() {
            press(&mut state, &mut context, KeyCode::Char(c));
        }
        press_mod(
            &mut state,
            &mut context,
            KeyCode::Char('z'),
            KeyModifiers::CONTROL,
        );
        assert_eq!(editor(&state).lines, vec![String::new()]);
        assert!(!editor(&state).is_dirty());
        press_mod(
            &mut state,
            &mut context,
            KeyCode::Char('y'),
            KeyModifiers::CONTROL,
        );
        assert_eq!(editor(&state).lines, vec!["hi".to_string()]);
    }

    #[test]
    fn f2_saves_and_shift_f2_opens_save_as() {
        let (mut state, mut context, _dir) = setup();
        press(&mut state, &mut context, KeyCode::Char('a'));
        press(&mut state, &mut context, KeyCode::F(2));
        let path = editor(&state).path.clone();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "a");
        assert!(!editor(&state).is_dirty());
        press_mod(&mut state, &mut context, KeyCode::F(2), KeyModifiers::SHIFT);
        assert!(matches!(
            state.dialogs.top(),
            Some(PopupType::EditorSaveAsPrompt { .. })
        ));
    }
}
