use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, Screen};
use crate::app::sys_helpers::find_next_in_editor;
use crate::app::text_input;
use crate::config::localization::t;
use crate::config::write_atomic;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_editor_screen(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<(), ()> {
    let term_height = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
    let edit_height = ((term_height as u16 * 90 / 100).saturating_sub(3)) as usize;

    let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);

    // Some global keys should still pass through like F12, Ctrl+Tab
    if key.code == KeyCode::F(12) || (key.code == KeyCode::Tab && is_ctrl) {
        return Err(()); // pass to global resolver
    }

    if let Some(Screen::Editor(ed)) = state.screens.get_mut(state.active_screen_idx) {
        match key.code {
            KeyCode::Char(c) if !is_ctrl => {
                if ed.lines.is_empty() {
                    ed.lines.push(String::new());
                }
                text_input::insert_char(&mut ed.lines[ed.cursor_y], &mut ed.cursor_x, c);
                ed.is_dirty = true;
            }
            KeyCode::Backspace => {
                if ed.cursor_x > 0 {
                    text_input::backspace(&mut ed.lines[ed.cursor_y], &mut ed.cursor_x);
                    ed.is_dirty = true;
                } else if ed.cursor_y > 0 {
                    let current_line = ed.lines.remove(ed.cursor_y);
                    ed.cursor_y -= 1;
                    let prev_line_len = ed.lines[ed.cursor_y].len();
                    ed.lines[ed.cursor_y].push_str(&current_line);
                    ed.cursor_x = prev_line_len;
                    ed.is_dirty = true;
                }
            }
            KeyCode::Delete => {
                if ed.cursor_y < ed.lines.len() {
                    if ed.cursor_x < ed.lines[ed.cursor_y].len() {
                        text_input::delete(&mut ed.lines[ed.cursor_y], &mut ed.cursor_x);
                        ed.is_dirty = true;
                    } else if ed.cursor_y < ed.lines.len() - 1 {
                        let next_line = ed.lines.remove(ed.cursor_y + 1);
                        ed.lines[ed.cursor_y].push_str(&next_line);
                        ed.is_dirty = true;
                    }
                }
            }
            KeyCode::Enter => {
                if ed.lines.is_empty() {
                    ed.lines.push(String::new());
                }
                let current_line = &mut ed.lines[ed.cursor_y];
                ed.cursor_x = text_input::floor_boundary(current_line, ed.cursor_x);
                let next_line = current_line.split_off(ed.cursor_x);
                ed.lines.insert(ed.cursor_y + 1, next_line);
                ed.cursor_y += 1;
                ed.cursor_x = 0;
                ed.is_dirty = true;
            }
            KeyCode::Up => {
                if ed.cursor_y > 0 {
                    ed.cursor_y -= 1;
                    ed.cursor_x = text_input::floor_boundary(&ed.lines[ed.cursor_y], ed.cursor_x);
                    if ed.cursor_y < ed.scroll_y {
                        ed.scroll_y = ed.cursor_y;
                    }
                }
            }
            KeyCode::Down => {
                if ed.cursor_y < ed.lines.len().saturating_sub(1) {
                    ed.cursor_y += 1;
                    ed.cursor_x = text_input::floor_boundary(&ed.lines[ed.cursor_y], ed.cursor_x);
                    if ed.cursor_y >= ed.scroll_y + edit_height {
                        ed.scroll_y = ed.cursor_y.saturating_sub(edit_height - 1);
                    }
                }
            }
            KeyCode::PageUp => {
                ed.cursor_y = ed.cursor_y.saturating_sub(edit_height);
                ed.cursor_x = text_input::floor_boundary(&ed.lines[ed.cursor_y], ed.cursor_x);
                if ed.cursor_y < ed.scroll_y {
                    ed.scroll_y = ed.cursor_y;
                }
            }
            KeyCode::PageDown => {
                ed.cursor_y = (ed.cursor_y + edit_height).min(ed.lines.len().saturating_sub(1));
                ed.cursor_x = text_input::floor_boundary(&ed.lines[ed.cursor_y], ed.cursor_x);
                if ed.cursor_y >= ed.scroll_y + edit_height {
                    ed.scroll_y = ed.cursor_y.saturating_sub(edit_height - 1);
                }
            }
            KeyCode::Left => {
                if ed.cursor_x > 0 {
                    ed.cursor_x = text_input::prev_boundary(&ed.lines[ed.cursor_y], ed.cursor_x);
                } else if ed.cursor_y > 0 {
                    ed.cursor_y -= 1;
                    ed.cursor_x = ed.lines[ed.cursor_y].len();
                }
            }
            KeyCode::Right => {
                if ed.cursor_y < ed.lines.len() {
                    let line_len = ed.lines[ed.cursor_y].len();
                    if ed.cursor_x < line_len {
                        ed.cursor_x =
                            text_input::next_boundary(&ed.lines[ed.cursor_y], ed.cursor_x);
                    } else if ed.cursor_y < ed.lines.len() - 1 {
                        ed.cursor_y += 1;
                        ed.cursor_x = 0;
                    }
                }
            }
            KeyCode::F(2) => {
                let content = ed.lines.join("\n");
                if let Err(e) = write_atomic(&ed.path, content.as_bytes()) {
                    state.dialogs.replace(PopupType::Error(
                        t("error_save_failed").replace("{}", &e.to_string()),
                    ));
                    return Ok(());
                }
                ed.is_dirty = false;
            }
            KeyCode::Char('s') if is_ctrl => {
                let content = ed.lines.join("\n");
                if let Err(e) = write_atomic(&ed.path, content.as_bytes()) {
                    state.dialogs.replace(PopupType::Error(
                        t("error_save_failed").replace("{}", &e.to_string()),
                    ));
                    return Ok(());
                }
                ed.is_dirty = false;
            }
            KeyCode::Char('r') | KeyCode::Char('d') if is_ctrl => {
                if context
                    .config
                    .settings
                    .confirmations
                    .confirm_reload_edited_file
                {
                    state.dialogs.replace(PopupType::ConfirmReload);
                    return Ok(());
                } else {
                    match std::fs::read_to_string(&ed.path) {
                        Ok(content) => {
                            let reloaded_lines: Vec<String> =
                                content.lines().map(|s| s.to_string()).collect();
                            ed.lines = if reloaded_lines.is_empty() {
                                vec![String::new()]
                            } else {
                                reloaded_lines
                            };
                            // Both cursor axes must be re-clamped to the
                            // new line count / line length, otherwise a
                            // shorter reloaded file would leave the
                            // editor in a state where cursor_y points
                            // past the end and the next keystroke would
                            // panic on `ed.lines[ed.cursor_y]`.
                            if ed.cursor_y >= ed.lines.len() {
                                ed.cursor_y = ed.lines.len() - 1;
                            }
                            ed.cursor_x =
                                text_input::floor_boundary(&ed.lines[ed.cursor_y], ed.cursor_x);
                            ed.is_dirty = false;
                        }
                        Err(e) => {
                            state.dialogs.replace(PopupType::Error(
                                t("error_reload_file_failed").replace("{}", &e.to_string()),
                            ));
                            return Ok(());
                        }
                    }
                }
            }
            KeyCode::F(7) if is_shift => {
                if let Some(ref q) = ed.last_search
                    && let Some((found_x, found_y)) = find_next_in_editor(
                        &ed.lines,
                        ed.cursor_x,
                        ed.cursor_y,
                        q,
                        ed.last_case_sensitive,
                    )
                {
                    ed.cursor_x = found_x;
                    ed.cursor_y = found_y;
                    if ed.cursor_y < ed.scroll_y || ed.cursor_y >= ed.scroll_y + edit_height {
                        ed.scroll_y = ed.cursor_y.saturating_sub(edit_height / 2);
                    }
                }
            }
            KeyCode::F(7) | KeyCode::Char('f') if is_ctrl || key.code == KeyCode::F(7) => {
                state.dialogs.replace(PopupType::EditorSearchPrompt {
                    query: String::new(),
                    case_sensitive: false,
                    cursor_idx: 0,
                });
                return Ok(());
            }
            KeyCode::F(3) => {
                if let Some(ref q) = ed.last_search
                    && let Some((found_x, found_y)) = find_next_in_editor(
                        &ed.lines,
                        ed.cursor_x,
                        ed.cursor_y,
                        q,
                        ed.last_case_sensitive,
                    )
                {
                    ed.cursor_x = found_x;
                    ed.cursor_y = found_y;
                    if ed.cursor_y < ed.scroll_y || ed.cursor_y >= ed.scroll_y + edit_height {
                        ed.scroll_y = ed.cursor_y.saturating_sub(edit_height / 2);
                    }
                }
            }
            KeyCode::F(4) => {
                let path = ed.path.clone();
                let mut viewer_state = crate::ui::viewer::ViewerState::load_with_images(
                    path,
                    context.config.settings.image_preview_enabled,
                );
                viewer_state.mode = crate::ui::viewer::ViewerMode::Hex;
                state.push_screen(Screen::Viewer(viewer_state));
                return Ok(());
            }
            KeyCode::F(8) => {
                state
                    .dialogs
                    .replace(PopupType::ConfirmDiscardEditorChanges);
                return Ok(());
            }
            KeyCode::Esc | KeyCode::F(10) => {
                if ed.is_dirty {
                    state
                        .dialogs
                        .replace(PopupType::ConfirmDiscardEditorChanges);
                } else {
                    state.close_current_screen();
                }
                return Ok(());
            }
            _ => {}
        }
        return Ok(());
    }
    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::types::EditorState;
    use crate::config::AppConfig;
    use std::path::PathBuf;

    fn setup() -> (AppState, AppContext) {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.push_screen(Screen::Editor(EditorState {
            path: PathBuf::from("unused.txt"),
            lines: vec![String::new()],
            cursor_x: 0,
            cursor_y: 0,
            scroll_y: 0,
            is_dirty: false,
            last_search: None,
            last_case_sensitive: false,
        }));
        let context = AppContext::new(AppConfig {
            settings: crate::config::settings::Settings::default(),
            theme: crate::config::theme::Theme::default(),
            keybindings: crate::config::keybindings::KeybindingsConfig::default(),
        });
        (state, context)
    }

    fn press(state: &mut AppState, context: &mut AppContext, code: KeyCode) {
        let key = KeyEvent::new(code, KeyModifiers::NONE);
        handle_editor_screen(state, key, context).expect("editor handles key");
    }

    fn editor(state: &AppState) -> &EditorState {
        match state.screens.get(state.active_screen_idx) {
            Some(Screen::Editor(ed)) => ed,
            _ => panic!("expected editor screen"),
        }
    }

    #[test]
    fn typing_and_editing_non_ascii_text() {
        let (mut state, mut context) = setup();
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
}
