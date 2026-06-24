use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.active_popup.clone();
    if let Some(PopupType::Help {
        mode,
        docs,
        mut cursor_idx,
        mut scroll_y,
        mut active_content,
    }) = popup
    {
        match key.code {
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Tab => {
                let new_mode = if mode == 0 { 1 } else { 0 };
                state.active_popup = Some(PopupType::Help {
                    mode: new_mode,
                    docs,
                    cursor_idx,
                    scroll_y,
                    active_content,
                });
                return Ok(None);
            }
            _ => {}
        }

        if mode == 0 {
            // Mode 0: Navigating Document Selection List
            match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    if !docs.is_empty() {
                        if cursor_idx == 0 {
                            cursor_idx = docs.len() - 1;
                        } else {
                            cursor_idx -= 1;
                        }
                        let path = &docs[cursor_idx].1;
                        active_content = std::fs::read_to_string(path).ok();
                        scroll_y = 0;
                    }
                    state.active_popup = Some(PopupType::Help {
                        mode,
                        docs,
                        cursor_idx,
                        scroll_y,
                        active_content,
                    });
                    Ok(None)
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    if !docs.is_empty() {
                        if cursor_idx + 1 >= docs.len() {
                            cursor_idx = 0;
                        } else {
                            cursor_idx += 1;
                        }
                        let path = &docs[cursor_idx].1;
                        active_content = std::fs::read_to_string(path).ok();
                        scroll_y = 0;
                    }
                    state.active_popup = Some(PopupType::Help {
                        mode,
                        docs,
                        cursor_idx,
                        scroll_y,
                        active_content,
                    });
                    Ok(None)
                }
                KeyCode::Enter => {
                    // Switch focus to right pane
                    state.active_popup = Some(PopupType::Help {
                        mode: 1,
                        docs,
                        cursor_idx,
                        scroll_y,
                        active_content,
                    });
                    Ok(None)
                }
                _ => Err(()),
            }
        } else {
            // Mode 1: Scrolling Document Content
            match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    if scroll_y > 0 {
                        scroll_y -= 1;
                    }
                    state.active_popup = Some(PopupType::Help {
                        mode,
                        docs,
                        cursor_idx,
                        scroll_y,
                        active_content,
                    });
                    Ok(None)
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    scroll_y += 1;
                    state.active_popup = Some(PopupType::Help {
                        mode,
                        docs,
                        cursor_idx,
                        scroll_y,
                        active_content,
                    });
                    Ok(None)
                }
                KeyCode::PageUp => {
                    if scroll_y >= 15 {
                        scroll_y -= 15;
                    } else {
                        scroll_y = 0;
                    }
                    state.active_popup = Some(PopupType::Help {
                        mode,
                        docs,
                        cursor_idx,
                        scroll_y,
                        active_content,
                    });
                    Ok(None)
                }
                KeyCode::PageDown => {
                    scroll_y += 15;
                    state.active_popup = Some(PopupType::Help {
                        mode,
                        docs,
                        cursor_idx,
                        scroll_y,
                        active_content,
                    });
                    Ok(None)
                }
                KeyCode::Backspace => {
                    // Backspace returns to list pane
                    state.active_popup = Some(PopupType::Help {
                        mode: 0,
                        docs,
                        cursor_idx,
                        scroll_y,
                        active_content,
                    });
                    Ok(None)
                }
                _ => Err(()),
            }
        }
    } else {
        Err(())
    }
}
