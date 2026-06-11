use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, Screen};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.active_popup.clone();
    if let Some(p) = popup {
        match p {
            PopupType::ScreensMenu {
                mut cursor_idx,
                suspended_popup,
            } => {
                match key.code {
                    KeyCode::Up => {
                        if cursor_idx > 0 {
                            cursor_idx -= 1;
                        } else {
                            cursor_idx = state.screens.len().saturating_sub(1);
                        }
                    }
                    KeyCode::Down => {
                        if cursor_idx < state.screens.len().saturating_sub(1) {
                            cursor_idx += 1;
                        } else {
                            cursor_idx = 0;
                        }
                    }
                    KeyCode::Enter => {
                        if cursor_idx < state.screens.len() {
                            // save current screen's popup if not staying on same screen
                            if cursor_idx != state.active_screen_idx {
                                state.screen_popups[state.active_screen_idx] =
                                    suspended_popup.map(|b| *b);
                                state.active_screen_idx = cursor_idx;
                                state.active_popup = state.screen_popups[cursor_idx].take();
                            } else {
                                state.active_popup = suspended_popup.map(|b| *b);
                            }
                        } else {
                            state.active_popup = None;
                        }
                        return Ok(None);
                    }
                    KeyCode::F(3) => {
                        if cursor_idx < state.screens.len() {
                            if let Screen::Terminal(ref ts) = state.screens[cursor_idx] {
                                let lines = ts.output_lines.clone();
                                let raw = ts.output_lines.join("\n").into_bytes();
                                let vw = crate::ui::viewer::ViewerState {
                                    path: std::path::PathBuf::from(format!(
                                        "Terminal: {}",
                                        ts.command
                                    )),
                                    lines,
                                    raw,
                                    image_data: None,
                                    is_image: false,
                                    is_text: true,
                                    mode: crate::ui::viewer::ViewerMode::Text,
                                    scroll: 0,
                                    last_search: None,
                                };
                                state.push_screen(Screen::Viewer(vw));
                                state.active_popup = None;
                                return Ok(None);
                            }
                        }
                    }
                    KeyCode::Esc | KeyCode::F(12) => {
                        state.active_popup = suspended_popup.map(|b| *b);
                        return Ok(None);
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::ScreensMenu {
                    cursor_idx,
                    suspended_popup,
                });
                Ok(None)
            }
            _ => Err(()),
        }
    } else {
        Err(())
    }
}
