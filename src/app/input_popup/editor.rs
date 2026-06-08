use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::app::sys_helpers::find_next_in_editor;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.active_popup.clone();
    if let Some(p) = popup {
        match p {
            PopupType::InternalEditor {
                path,
                mut lines,
                mut cursor_x,
                mut cursor_y,
                mut scroll_y,
                mut is_dirty,
                last_search,
            } => {
                let term_height = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
                let edit_height = ((term_height as u16 * 90 / 100).saturating_sub(3)) as usize;

                let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);

                match key.code {
                    KeyCode::Char(c) if !is_ctrl => {
                        if lines.is_empty() {
                            lines.push(String::new());
                        }
                        let line = &mut lines[cursor_y];
                        if cursor_x <= line.len() {
                            line.insert(cursor_x, c);
                            cursor_x += 1;
                            is_dirty = true;
                        }
                    }
                    KeyCode::Backspace => {
                        if cursor_x > 0 {
                            let line = &mut lines[cursor_y];
                            line.remove(cursor_x - 1);
                            cursor_x -= 1;
                            is_dirty = true;
                        } else if cursor_y > 0 {
                            let current_line = lines.remove(cursor_y);
                            cursor_y -= 1;
                            let prev_line_len = lines[cursor_y].len();
                            lines[cursor_y].push_str(&current_line);
                            cursor_x = prev_line_len;
                            is_dirty = true;
                        }
                    }
                    KeyCode::Delete => {
                        if cursor_y < lines.len() {
                            let line = &mut lines[cursor_y];
                            if cursor_x < line.len() {
                                line.remove(cursor_x);
                                is_dirty = true;
                            } else if cursor_y < lines.len() - 1 {
                                let next_line = lines.remove(cursor_y + 1);
                                lines[cursor_y].push_str(&next_line);
                                is_dirty = true;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if lines.is_empty() {
                            lines.push(String::new());
                        }
                        let current_line = &mut lines[cursor_y];
                        let next_line = current_line.split_off(cursor_x);
                        lines.insert(cursor_y + 1, next_line);
                        cursor_y += 1;
                        cursor_x = 0;
                        is_dirty = true;
                    }
                    KeyCode::Up => {
                        if cursor_y > 0 {
                            cursor_y -= 1;
                            cursor_x = cursor_x.min(lines[cursor_y].len());
                            if cursor_y < scroll_y {
                                scroll_y = cursor_y;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if cursor_y < lines.len().saturating_sub(1) {
                            cursor_y += 1;
                            cursor_x = cursor_x.min(lines[cursor_y].len());
                            if cursor_y >= scroll_y + edit_height {
                                scroll_y = cursor_y.saturating_sub(edit_height - 1);
                            }
                        }
                    }
                    KeyCode::PageUp => {
                        cursor_y = cursor_y.saturating_sub(edit_height);
                        cursor_x = cursor_x.min(lines[cursor_y].len());
                        if cursor_y < scroll_y {
                            scroll_y = cursor_y;
                        }
                    }
                    KeyCode::PageDown => {
                        cursor_y = (cursor_y + edit_height).min(lines.len().saturating_sub(1));
                        cursor_x = cursor_x.min(lines[cursor_y].len());
                        if cursor_y >= scroll_y + edit_height {
                            scroll_y = cursor_y.saturating_sub(edit_height - 1);
                        }
                    }
                    KeyCode::Left => {
                        if cursor_x > 0 {
                            cursor_x -= 1;
                        } else if cursor_y > 0 {
                            cursor_y -= 1;
                            cursor_x = lines[cursor_y].len();
                        }
                    }
                    KeyCode::Right => {
                        if cursor_y < lines.len() {
                            let line_len = lines[cursor_y].len();
                            if cursor_x < line_len {
                                cursor_x += 1;
                            } else if cursor_y < lines.len() - 1 {
                                cursor_y += 1;
                                cursor_x = 0;
                            }
                        }
                    }
                    KeyCode::F(2) => {
                        let content = lines.join("\n");
                        if let Err(e) = std::fs::write(&path, content) {
                            state.active_popup =
                                Some(PopupType::Error(format!("Failed to save: {}", e)));
                            return Ok(None);
                        }
                        is_dirty = false;
                    }
                    KeyCode::Char('s') if is_ctrl => {
                        let content = lines.join("\n");
                        if let Err(e) = std::fs::write(&path, content) {
                            state.active_popup =
                                Some(PopupType::Error(format!("Failed to save: {}", e)));
                            return Ok(None);
                        }
                        is_dirty = false;
                    }
                    KeyCode::Char('r') | KeyCode::Char('d') if is_ctrl => {
                        if context.config.settings.confirmations.confirm_reload_edited_file {
                            state.active_popup = Some(PopupType::ConfirmReload {
                                path: path.clone(),
                                lines: lines.clone(),
                                cursor_x,
                                cursor_y,
                                scroll_y,
                                is_dirty,
                                last_search,
                            });
                            return Ok(None);
                        } else {
                            match std::fs::read_to_string(&path) {
                                Ok(content) => {
                                    let reloaded_lines: Vec<String> =
                                        content.lines().map(|s| s.to_string()).collect();
                                    lines = if reloaded_lines.is_empty() {
                                        vec![String::new()]
                                    } else {
                                        reloaded_lines
                                    };
                                    cursor_x =
                                        cursor_x.min(lines.get(cursor_y).map(|l| l.len()).unwrap_or(0));
                                    is_dirty = false;
                                }
                                Err(e) => {
                                    state.active_popup =
                                        Some(PopupType::Error(format!("Failed to reload: {}", e)));
                                    return Ok(None);
                                }
                            }
                        }
                    },
                    KeyCode::F(7) if is_shift => {
                        if let Some(ref q) = last_search {
                            if let Some((found_x, found_y)) =
                                find_next_in_editor(&lines, cursor_x, cursor_y, q)
                            {
                                cursor_x = found_x;
                                cursor_y = found_y;
                                if cursor_y < scroll_y || cursor_y >= scroll_y + edit_height {
                                    scroll_y = cursor_y.saturating_sub(edit_height / 2);
                                }
                            }
                        }
                    }
                    KeyCode::F(7) | KeyCode::Char('f') if is_ctrl || key.code == KeyCode::F(7) => {
                        state.active_popup = Some(PopupType::EditorSearchPrompt {
                            path,
                            lines,
                            cursor_x,
                            cursor_y,
                            scroll_y,
                            is_dirty,
                            last_search,
                            query: String::new(),
                        });
                        return Ok(None);
                    }
                    KeyCode::F(3) => {
                        if let Some(ref q) = last_search {
                            if let Some((found_x, found_y)) =
                                find_next_in_editor(&lines, cursor_x, cursor_y, q)
                            {
                                cursor_x = found_x;
                                cursor_y = found_y;
                                if cursor_y < scroll_y || cursor_y >= scroll_y + edit_height {
                                    scroll_y = cursor_y.saturating_sub(edit_height / 2);
                                }
                            }
                        }
                    }
                    KeyCode::Esc | KeyCode::F(10) => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::InternalEditor {
                    path,
                    lines,
                    cursor_x,
                    cursor_y,
                    scroll_y,
                    is_dirty,
                    last_search,
                });
                Ok(None)
            }
            PopupType::EditorSearchPrompt {
                path,
                lines,
                cursor_x,
                cursor_y,
                scroll_y,
                is_dirty,
                last_search,
                mut query,
            } => {
                let term_height = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
                let edit_height = ((term_height as u16 * 90 / 100).saturating_sub(3)) as usize;

                let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                match key.code {
                    KeyCode::Char(c) if !is_ctrl => {
                        query.push(c);
                    }
                    KeyCode::Backspace => {
                        query.pop();
                    }
                    KeyCode::Esc => {
                        state.active_popup = Some(PopupType::InternalEditor {
                            path,
                            lines,
                            cursor_x,
                            cursor_y,
                            scroll_y,
                            is_dirty,
                            last_search,
                        });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        let q = query.clone();
                        if !q.is_empty() {
                            if let Some((found_x, found_y)) =
                                find_next_in_editor(&lines, cursor_x, cursor_y, &q)
                            {
                                let new_cursor_x = found_x;
                                let new_cursor_y = found_y;
                                let mut new_scroll_y = scroll_y;
                                if new_cursor_y < new_scroll_y
                                    || new_cursor_y >= new_scroll_y + edit_height
                                {
                                    new_scroll_y = new_cursor_y.saturating_sub(edit_height / 2);
                                }
                                state.active_popup = Some(PopupType::InternalEditor {
                                    path,
                                    lines,
                                    cursor_x: new_cursor_x,
                                    cursor_y: new_cursor_y,
                                    scroll_y: new_scroll_y,
                                    is_dirty,
                                    last_search: Some(q),
                                });
                            } else {
                                // Show "Text not found" popup message to satisfy the request.
                                state.active_popup =
                                    Some(PopupType::Error("Text not found".to_string()));
                            }
                        } else {
                            state.active_popup = Some(PopupType::InternalEditor {
                                path,
                                lines,
                                cursor_x,
                                cursor_y,
                                scroll_y,
                                is_dirty,
                                last_search,
                            });
                        }
                        return Ok(None);
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::EditorSearchPrompt {
                    path,
                    lines,
                    cursor_x,
                    cursor_y,
                    scroll_y,
                    is_dirty,
                    last_search,
                    query,
                });
                Ok(None)
            }
            _ => Err(()),
        }
    } else {
        Err(())
    }
}
