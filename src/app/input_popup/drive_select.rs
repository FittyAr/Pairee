use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::DriveSelect {
        panel,
        drives,
        cursor_idx,
    }) = state.active_popup.clone()
    {
        match key.code {
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Up => {
                if !drives.is_empty() {
                    let new_idx = if cursor_idx > 0 {
                        cursor_idx - 1
                    } else {
                        drives.len() - 1
                    };
                    state.active_popup = Some(PopupType::DriveSelect {
                        panel,
                        drives,
                        cursor_idx: new_idx,
                    });
                }
                return Ok(None);
            }
            KeyCode::Down => {
                if !drives.is_empty() {
                    let new_idx = if cursor_idx < drives.len() - 1 {
                        cursor_idx + 1
                    } else {
                        0
                    };
                    state.active_popup = Some(PopupType::DriveSelect {
                        panel,
                        drives,
                        cursor_idx: new_idx,
                    });
                }
                return Ok(None);
            }
            KeyCode::Enter => {
                if let Some(drive_path) = drives.get(cursor_idx) {
                    let target_path = std::path::PathBuf::from(drive_path);
                    match panel {
                        ActivePanel::Left => {
                            state.left_panel.current_path = target_path;
                            state.left_panel.cursor_index = 0;
                            state.left_panel.clear_selection();
                        }
                        ActivePanel::Right => {
                            state.right_panel.current_path = target_path;
                            state.right_panel.cursor_index = 0;
                            state.right_panel.clear_selection();
                        }
                    }
                    state.active_popup = None;
                    state.refresh_both_panels(context.config.settings.show_hidden);
                }
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
