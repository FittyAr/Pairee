use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::Hotlist {
        bookmarks,
        cursor_idx,
    }) = state.active_popup.clone()
    {
        match key.code {
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Up => {
                if !bookmarks.is_empty() {
                    let new_idx = if cursor_idx > 0 {
                        cursor_idx - 1
                    } else {
                        bookmarks.len() - 1
                    };
                    state.active_popup = Some(PopupType::Hotlist {
                        bookmarks,
                        cursor_idx: new_idx,
                    });
                }
                return Ok(None);
            }
            KeyCode::Down => {
                if !bookmarks.is_empty() {
                    let new_idx = if cursor_idx < bookmarks.len() - 1 {
                        cursor_idx + 1
                    } else {
                        0
                    };
                    state.active_popup = Some(PopupType::Hotlist {
                        bookmarks,
                        cursor_idx: new_idx,
                    });
                }
                return Ok(None);
            }
            KeyCode::Enter => {
                if let Some((_, target_path)) = bookmarks.get(cursor_idx) {
                    let panel = state.get_active_panel_mut();
                    panel.current_path = target_path.clone();
                    panel.cursor_index = 0;
                    panel.clear_selection();
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
