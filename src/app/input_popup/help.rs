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
        active_content,
    }) = popup
    {
        if mode == 0 {
            // Mode 0: Navigating Document Selection List
            match key.code {
                KeyCode::Up => {
                    if !docs.is_empty() {
                        if cursor_idx == 0 {
                            cursor_idx = docs.len() - 1;
                        } else {
                            cursor_idx -= 1;
                        }
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
                KeyCode::Down => {
                    if !docs.is_empty() {
                        if cursor_idx + 1 >= docs.len() {
                            cursor_idx = 0;
                        } else {
                            cursor_idx += 1;
                        }
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
                    if cursor_idx < docs.len() {
                        let path = &docs[cursor_idx].1;
                        let content = std::fs::read_to_string(path)
                            .unwrap_or_else(|e| format!("Failed to read file: {}\nError: {}", path.display(), e));
                        state.active_popup = Some(PopupType::Help {
                            mode: 1,
                            docs,
                            cursor_idx,
                            scroll_y: 0,
                            active_content: Some(content),
                        });
                    }
                    Ok(None)
                }
                KeyCode::Esc => {
                    state.active_popup = None;
                    Ok(None)
                }
                _ => Err(()),
            }
        } else {
            // Mode 1: Reading Open Markdown Document
            match key.code {
                KeyCode::Up => {
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
                KeyCode::Down => {
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
                KeyCode::Esc | KeyCode::Backspace => {
                    // Go back to document list
                    state.active_popup = Some(PopupType::Help {
                        mode: 0,
                        docs,
                        cursor_idx,
                        scroll_y: 0,
                        active_content: None,
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
