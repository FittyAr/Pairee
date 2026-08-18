use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, Screen};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.dialogs.top().cloned();
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
                                let mut parked = crate::app::state::DialogStack::new();
                                if let Some(p) = suspended_popup {
                                    parked.replace(*p);
                                }
                                state.screen_dialogs[state.active_screen_idx] = parked;
                                state.active_screen_idx = cursor_idx;
                                state.dialogs =
                                    std::mem::take(&mut state.screen_dialogs[cursor_idx]);
                            } else {
                                state.dialogs.set(suspended_popup.map(|b| *b));
                            }
                        } else {
                            state.dialogs.clear();
                        }
                        return Ok(None);
                    }
                    KeyCode::F(3) => {
                        if cursor_idx < state.screens.len()
                            && let Screen::Terminal(ref ts) = state.screens[cursor_idx]
                        {
                            let lines = ts.output_lines.clone();
                            let raw = ts.output_lines.join("\n").into_bytes();
                            let vw = crate::ui::viewer::ViewerState {
                                path: std::path::PathBuf::from(format!("Terminal: {}", ts.command)),
                                lines,
                                raw,
                                image_data: None,
                                is_image: false,
                                is_text: true,
                                mode: crate::ui::viewer::ViewerMode::Text,
                                scroll: 0,
                                last_search: None,
                                last_case_sensitive: false,
                            };
                            state.push_screen(Screen::Viewer(vw));
                            state.dialogs.clear();
                            return Ok(None);
                        }
                    }
                    KeyCode::Esc | KeyCode::F(12) => {
                        state.dialogs.set(suspended_popup.map(|b| *b));
                        return Ok(None);
                    }
                    _ => {}
                }
                state.dialogs.replace(PopupType::ScreensMenu {
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
