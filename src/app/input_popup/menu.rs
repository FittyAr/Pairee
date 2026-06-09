use crate::app::context::AppContext;
use crate::app::menu_handler::trigger_menu_item;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::Menu {
        active_menu_idx,
        active_item_idx,
    }) = state.active_popup
    {
        let items = crate::ui::menu::get_menu_items(active_menu_idx, state);
        match key.code {
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Left => {
                let new_idx = if active_menu_idx > 0 {
                    active_menu_idx - 1
                } else {
                    4
                };
                state.active_popup = Some(PopupType::Menu {
                    active_menu_idx: new_idx,
                    active_item_idx: 0,
                });
                return Ok(None);
            }
            KeyCode::Right => {
                let new_idx = if active_menu_idx < 4 {
                    active_menu_idx + 1
                } else {
                    0
                };
                state.active_popup = Some(PopupType::Menu {
                    active_menu_idx: new_idx,
                    active_item_idx: 0,
                });
                return Ok(None);
            }
            KeyCode::Up => {
                if !items.is_empty() {
                    let new_item_idx = if active_item_idx > 0 {
                        active_item_idx - 1
                    } else {
                        items.len() - 1
                    };
                    state.active_popup = Some(PopupType::Menu {
                        active_menu_idx,
                        active_item_idx: new_item_idx,
                    });
                }
                return Ok(None);
            }
            KeyCode::Down => {
                if !items.is_empty() {
                    let new_item_idx = if active_item_idx < items.len() - 1 {
                        active_item_idx + 1
                    } else {
                        0
                    };
                    state.active_popup = Some(PopupType::Menu {
                        active_menu_idx,
                        active_item_idx: new_item_idx,
                    });
                }
                return Ok(None);
            }
            KeyCode::Enter => {
                state.active_popup = None;
                let action = trigger_menu_item(state, context, active_menu_idx, active_item_idx);
                return Ok(action);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
