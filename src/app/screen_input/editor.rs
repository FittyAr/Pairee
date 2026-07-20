use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, Screen};
use crate::app::sys_helpers::find_next_in_editor;
use crate::config::localization::t;
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
                let line = &mut ed.lines[ed.cursor_y];
                if ed.cursor_x <= line.len() {
                    line.insert(ed.cursor_x, c);
                    ed.cursor_x += 1;
                    ed.is_dirty = true;
                }
            }
            KeyCode::Backspace => {
                if ed.cursor_x > 0 {
                    let line = &mut ed.lines[ed.cursor_y];
                    line.remove(ed.cursor_x - 1);
                    ed.cursor_x -= 1;
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
                    let line = &mut ed.lines[ed.cursor_y];
                    if ed.cursor_x < line.len() {
                        line.remove(ed.cursor_x);
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
                let next_line = current_line.split_off(ed.cursor_x);
                ed.lines.insert(ed.cursor_y + 1, next_line);
                ed.cursor_y += 1;
                ed.cursor_x = 0;
                ed.is_dirty = true;
            }
            KeyCode::Up => {
                if ed.cursor_y > 0 {
                    ed.cursor_y -= 1;
                    ed.cursor_x = ed.cursor_x.min(ed.lines[ed.cursor_y].len());
                    if ed.cursor_y < ed.scroll_y {
                        ed.scroll_y = ed.cursor_y;
                    }
                }
            }
            KeyCode::Down => {
                if ed.cursor_y < ed.lines.len().saturating_sub(1) {
                    ed.cursor_y += 1;
                    ed.cursor_x = ed.cursor_x.min(ed.lines[ed.cursor_y].len());
                    if ed.cursor_y >= ed.scroll_y + edit_height {
                        ed.scroll_y = ed.cursor_y.saturating_sub(edit_height - 1);
                    }
                }
            }
            KeyCode::PageUp => {
                ed.cursor_y = ed.cursor_y.saturating_sub(edit_height);
                ed.cursor_x = ed.cursor_x.min(ed.lines[ed.cursor_y].len());
                if ed.cursor_y < ed.scroll_y {
                    ed.scroll_y = ed.cursor_y;
                }
            }
            KeyCode::PageDown => {
                ed.cursor_y = (ed.cursor_y + edit_height).min(ed.lines.len().saturating_sub(1));
                ed.cursor_x = ed.cursor_x.min(ed.lines[ed.cursor_y].len());
                if ed.cursor_y >= ed.scroll_y + edit_height {
                    ed.scroll_y = ed.cursor_y.saturating_sub(edit_height - 1);
                }
            }
            KeyCode::Left => {
                if ed.cursor_x > 0 {
                    ed.cursor_x -= 1;
                } else if ed.cursor_y > 0 {
                    ed.cursor_y -= 1;
                    ed.cursor_x = ed.lines[ed.cursor_y].len();
                }
            }
            KeyCode::Right => {
                if ed.cursor_y < ed.lines.len() {
                    let line_len = ed.lines[ed.cursor_y].len();
                    if ed.cursor_x < line_len {
                        ed.cursor_x += 1;
                    } else if ed.cursor_y < ed.lines.len() - 1 {
                        ed.cursor_y += 1;
                        ed.cursor_x = 0;
                    }
                }
            }
            KeyCode::F(2) => {
                let content = ed.lines.join("\n");
                if let Err(e) = std::fs::write(&ed.path, content) {
                    state.active_popup = Some(PopupType::Error(
                        t("error_save_failed").replace("{}", &e.to_string()),
                    ));
                    return Ok(());
                }
                ed.is_dirty = false;
            }
            KeyCode::Char('s') if is_ctrl => {
                let content = ed.lines.join("\n");
                if let Err(e) = std::fs::write(&ed.path, content) {
                    state.active_popup = Some(PopupType::Error(
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
                    state.active_popup = Some(PopupType::ConfirmReload);
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
                            ed.cursor_x = ed
                                .cursor_x
                                .min(ed.lines.get(ed.cursor_y).map(|l| l.len()).unwrap_or(0));
                            ed.is_dirty = false;
                        }
                        Err(e) => {
                            state.active_popup = Some(PopupType::Error(
                                t("error_reload_file_failed").replace("{}", &e.to_string()),
                            ));
                            return Ok(());
                        }
                    }
                }
            }
            KeyCode::F(7) if is_shift => {
                if let Some(ref q) = ed.last_search {
                    if let Some((found_x, found_y)) = find_next_in_editor(
                        &ed.lines,
                        ed.cursor_x,
                        ed.cursor_y,
                        q,
                        ed.last_case_sensitive,
                    ) {
                        ed.cursor_x = found_x;
                        ed.cursor_y = found_y;
                        if ed.cursor_y < ed.scroll_y || ed.cursor_y >= ed.scroll_y + edit_height {
                            ed.scroll_y = ed.cursor_y.saturating_sub(edit_height / 2);
                        }
                    }
                }
            }
            KeyCode::F(7) | KeyCode::Char('f') if is_ctrl || key.code == KeyCode::F(7) => {
                state.active_popup = Some(PopupType::EditorSearchPrompt {
                    query: String::new(),
                    case_sensitive: false,
                    cursor_idx: 0,
                });
                return Ok(());
            }
            KeyCode::F(3) => {
                if let Some(ref q) = ed.last_search {
                    if let Some((found_x, found_y)) = find_next_in_editor(
                        &ed.lines,
                        ed.cursor_x,
                        ed.cursor_y,
                        q,
                        ed.last_case_sensitive,
                    ) {
                        ed.cursor_x = found_x;
                        ed.cursor_y = found_y;
                        if ed.cursor_y < ed.scroll_y || ed.cursor_y >= ed.scroll_y + edit_height {
                            ed.scroll_y = ed.cursor_y.saturating_sub(edit_height / 2);
                        }
                    }
                }
            }
            KeyCode::F(4) => {
                let path = ed.path.clone();
                let mut viewer_state = crate::ui::viewer::ViewerState::load(path);
                viewer_state.mode = crate::ui::viewer::ViewerMode::Hex;
                state.push_screen(Screen::Viewer(viewer_state));
                return Ok(());
            }
            KeyCode::F(8) => {
                state.active_popup = Some(PopupType::ConfirmDiscardEditorChanges);
                return Ok(());
            }
            KeyCode::Esc | KeyCode::F(10) => {
                if ed.is_dirty {
                    state.active_popup = Some(PopupType::ConfirmDiscardEditorChanges);
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
