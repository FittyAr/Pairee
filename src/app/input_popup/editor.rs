use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::app::sys_helpers::find_next_in_editor;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.active_popup.clone();
    if let Some(p) = popup {
        match p {
            PopupType::EditorSearchPrompt {
                mut query,
                mut case_sensitive,
                mut cursor_idx,
            } => {
                let term_height = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
                let edit_height = ((term_height as u16 * 90 / 100).saturating_sub(3)) as usize;

                let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                match key.code {
                    KeyCode::Tab | KeyCode::Down => {
                        cursor_idx = (cursor_idx + 1) % 4;
                    }
                    KeyCode::Up => {
                        cursor_idx = if cursor_idx == 0 { 3 } else { cursor_idx - 1 };
                    }
                    KeyCode::Left | KeyCode::Right => {
                        if cursor_idx == 2 || cursor_idx == 3 {
                            cursor_idx = if cursor_idx == 2 { 3 } else { 2 };
                        }
                    }
                    KeyCode::Char(c) => {
                        if cursor_idx == 0 && !is_ctrl {
                            query.push(c);
                        } else if cursor_idx == 1 && c == ' ' {
                            case_sensitive = !case_sensitive;
                        }
                    }
                    KeyCode::Backspace => {
                        if cursor_idx == 0 {
                            query.pop();
                        }
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        if cursor_idx == 3 {
                            state.active_popup = None;
                            return Ok(None);
                        }
                        let q = query.clone();
                        if !q.is_empty() {
                            if let Some(crate::app::state::Screen::Editor(ed)) =
                                state.screens.get_mut(state.active_screen_idx)
                            {
                                if let Some((found_x, found_y)) = find_next_in_editor(
                                    &ed.lines,
                                    ed.cursor_x,
                                    ed.cursor_y,
                                    &q,
                                    case_sensitive,
                                ) {
                                    ed.cursor_x = found_x;
                                    ed.cursor_y = found_y;
                                    if ed.cursor_y < ed.scroll_y
                                        || ed.cursor_y >= ed.scroll_y + edit_height
                                    {
                                        ed.scroll_y = ed.cursor_y.saturating_sub(edit_height / 2);
                                    }
                                    ed.last_search = Some(q.clone());
                                    ed.last_case_sensitive = case_sensitive;
                                    state.active_popup = Some(PopupType::EditorSearchPrompt {
                                        query,
                                        case_sensitive,
                                        cursor_idx,
                                    });
                                } else {
                                    state.active_popup =
                                        Some(PopupType::Error("Text not found".to_string()));
                                }
                            } else {
                                state.active_popup = None;
                            }
                        } else {
                            state.active_popup = None;
                        }
                        return Ok(None);
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::EditorSearchPrompt {
                    query,
                    case_sensitive,
                    cursor_idx,
                });
                Ok(None)
            }
            _ => Err(()),
        }
    } else {
        Err(())
    }
}
