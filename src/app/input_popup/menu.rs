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
        let items = crate::ui::menu::get_menu_items(
            active_menu_idx,
            state,
            &context.resolver,
            &context.config.settings,
        );
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
                let new_item_idx = if active_item_idx.is_some() {
                    Some(0)
                } else {
                    None
                };
                state.active_popup = Some(PopupType::Menu {
                    active_menu_idx: new_idx,
                    active_item_idx: new_item_idx,
                });
                return Ok(None);
            }
            KeyCode::Right => {
                let new_idx = if active_menu_idx < 4 {
                    active_menu_idx + 1
                } else {
                    0
                };
                let new_item_idx = if active_item_idx.is_some() {
                    Some(0)
                } else {
                    None
                };
                state.active_popup = Some(PopupType::Menu {
                    active_menu_idx: new_idx,
                    active_item_idx: new_item_idx,
                });
                return Ok(None);
            }
            KeyCode::Up => {
                if !items.is_empty() {
                    let mut new_item_idx = if let Some(idx) = active_item_idx {
                        if idx > 0 { idx - 1 } else { items.len() - 1 }
                    } else {
                        items.len() - 1
                    };
                    while items
                        .get(new_item_idx)
                        .map_or(false, |item| item.is_separator)
                    {
                        new_item_idx = if new_item_idx > 0 {
                            new_item_idx - 1
                        } else {
                            items.len() - 1
                        };
                    }
                    state.active_popup = Some(PopupType::Menu {
                        active_menu_idx,
                        active_item_idx: Some(new_item_idx),
                    });
                }
                return Ok(None);
            }
            KeyCode::Down => {
                if !items.is_empty() {
                    let mut new_item_idx = if let Some(idx) = active_item_idx {
                        if idx < items.len() - 1 { idx + 1 } else { 0 }
                    } else {
                        0
                    };
                    while items
                        .get(new_item_idx)
                        .map_or(false, |item| item.is_separator)
                    {
                        new_item_idx = if new_item_idx < items.len() - 1 {
                            new_item_idx + 1
                        } else {
                            0
                        };
                    }
                    state.active_popup = Some(PopupType::Menu {
                        active_menu_idx,
                        active_item_idx: Some(new_item_idx),
                    });
                }
                return Ok(None);
            }
            KeyCode::Enter => {
                if let Some(idx) = active_item_idx {
                    state.active_popup = None;
                    let action = trigger_menu_item(state, context, active_menu_idx, idx);
                    return Ok(action);
                } else {
                    state.active_popup = Some(PopupType::Menu {
                        active_menu_idx,
                        active_item_idx: Some(0),
                    });
                    return Ok(None);
                }
            }
            KeyCode::Char(c) => {
                let lower_c = c.to_ascii_lowercase();

                // 1. Check dropdown items only if dropdown is open
                if active_item_idx.is_some() {
                    for (i, item) in items.iter().enumerate() {
                        if item.is_separator {
                            continue;
                        }
                        let parsed = crate::ui::hotkey::parse_hotkey(&item.label);
                        if let Some(hotkey) = parsed.hotkey {
                            if hotkey == lower_c {
                                state.active_popup = None;
                                let action = trigger_menu_item(state, context, active_menu_idx, i);
                                return Ok(action);
                            }
                        }
                    }
                }

                // 2. Check top menu titles
                let titles = crate::ui::menu::get_menu_titles();
                for (i, title) in titles.iter().enumerate() {
                    let parsed = crate::ui::hotkey::parse_hotkey(&title);
                    if let Some(hotkey) = parsed.hotkey {
                        if hotkey == lower_c {
                            state.active_popup = Some(PopupType::Menu {
                                active_menu_idx: i,
                                active_item_idx: Some(0),
                            });
                            return Ok(None);
                        }
                    }
                }
                return Ok(None);
            }
            _ => return Ok(None), // Absorb any unmapped keys so the menu isn't closed and CLI doesn't get it
        }
    } else {
        Err(())
    }
}
