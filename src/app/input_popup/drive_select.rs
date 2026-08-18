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
    }) = state.dialogs.top().cloned()
    {
        match key.code {
            KeyCode::Esc => {
                state.dialogs.clear();
                return Ok(None);
            }
            KeyCode::Up => {
                if !drives.is_empty() {
                    let new_idx = if cursor_idx > 0 {
                        cursor_idx - 1
                    } else {
                        drives.len() - 1
                    };
                    state.dialogs.replace(PopupType::DriveSelect {
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
                    state.dialogs.replace(PopupType::DriveSelect {
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
                            state.panels.left.current_path = target_path;
                            state.panels.left.cursor_index = 0;
                            state.panels.left.clear_selection();
                        }
                        ActivePanel::Right => {
                            state.panels.right.current_path = target_path;
                            state.panels.right.cursor_index = 0;
                            state.panels.right.clear_selection();
                        }
                    }
                    state.dialogs.clear();
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
