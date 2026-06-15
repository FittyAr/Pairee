use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, Screen};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(popup) = state.active_popup.clone() {
        match popup {
            PopupType::ViewerSearchPrompt {
                mut query,
                mut case_sensitive,
                mut cursor_idx,
            } => {
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
                        if cursor_idx == 0 {
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

                        if !query.is_empty() {
                            if let Some(Screen::Viewer(vw)) =
                                state.screens.get_mut(state.active_screen_idx)
                            {
                                let is_repeat = vw.last_search.as_ref() == Some(&query);
                                let start_from = if is_repeat { vw.scroll + 1 } else { vw.scroll };

                                vw.last_search = Some(query.clone());
                                vw.last_case_sensitive = case_sensitive;
                                if vw.mode == crate::ui::viewer::ViewerMode::Text {
                                    let match_fn = |l: &str| {
                                        if case_sensitive {
                                            l.contains(&query)
                                        } else {
                                            l.to_lowercase().contains(&query.to_lowercase())
                                        }
                                    };
                                    // simple downward search from current line
                                    if let Some(found_idx) = vw
                                        .lines
                                        .iter()
                                        .enumerate()
                                        .skip(start_from)
                                        .find(|(_, l)| match_fn(l))
                                        .map(|(i, _)| i)
                                    {
                                        vw.scroll = found_idx;
                                    } else if let Some(found_idx) = vw
                                        .lines
                                        .iter()
                                        .enumerate()
                                        .take(start_from)
                                        .find(|(_, l)| match_fn(l))
                                        .map(|(i, _)| i)
                                    {
                                        vw.scroll = found_idx;
                                    }
                                }
                            }
                        }
                        state.active_popup = Some(PopupType::ViewerSearchPrompt {
                            query,
                            case_sensitive,
                            cursor_idx,
                        });
                        return Ok(None);
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::ViewerSearchPrompt {
                    query,
                    case_sensitive,
                    cursor_idx,
                });
                return Ok(None);
            }
            _ => {}
        }
    }
    Err(())
}
