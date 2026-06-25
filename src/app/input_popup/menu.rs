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
        active_submenu_idx,
        active_submenu_item_idx,
    }) = state.active_popup.clone()
    {
        // 1. Get the items currently being interacted with (submenu or main menu)
        let current_menu_idx = active_submenu_idx.unwrap_or(active_menu_idx);
        let current_item_idx = active_submenu_item_idx.or(active_item_idx);

        let items = crate::ui::menu::get_menu_items(
            current_menu_idx,
            state,
            &context.resolver,
            &context.config.settings,
        );

        match key.code {
            KeyCode::Esc => {
                if active_submenu_idx.is_some() {
                    // Close submenu, return to main menu
                    state.active_popup = Some(PopupType::Menu {
                        active_menu_idx,
                        active_item_idx,
                        active_submenu_idx: None,
                        active_submenu_item_idx: None,
                    });
                } else {
                    state.active_popup = None;
                }
                return Ok(None);
            }
            KeyCode::Left => {
                if active_submenu_idx.is_some() {
                    // Close submenu, return to main menu
                    state.active_popup = Some(PopupType::Menu {
                        active_menu_idx,
                        active_item_idx,
                        active_submenu_idx: None,
                        active_submenu_item_idx: None,
                    });
                } else {
                    // Normal main menu transition left
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
                        active_submenu_idx: None,
                        active_submenu_item_idx: None,
                    });
                }
                return Ok(None);
            }
            KeyCode::Right => {
                if active_submenu_idx.is_none() {
                    // If the current highlighted item has a submenu, open it!
                    if let Some(idx) = active_item_idx {
                        if let Some(item) = items.get(idx) {
                            if let Some(sub_idx) = item.submenu_idx {
                                state.active_popup = Some(PopupType::Menu {
                                    active_menu_idx,
                                    active_item_idx,
                                    active_submenu_idx: Some(sub_idx),
                                    active_submenu_item_idx: Some(0),
                                });
                                return Ok(None);
                            }
                        }
                    }

                    // Otherwise, move to next top-level menu
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
                        active_submenu_idx: None,
                        active_submenu_item_idx: None,
                    });
                }
                return Ok(None);
            }
            KeyCode::Up => {
                if !items.is_empty() {
                    let mut new_item_idx = if let Some(idx) = current_item_idx {
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
                    if active_submenu_idx.is_some() {
                        state.active_popup = Some(PopupType::Menu {
                            active_menu_idx,
                            active_item_idx,
                            active_submenu_idx,
                            active_submenu_item_idx: Some(new_item_idx),
                        });
                    } else {
                        state.active_popup = Some(PopupType::Menu {
                            active_menu_idx,
                            active_item_idx: Some(new_item_idx),
                            active_submenu_idx: None,
                            active_submenu_item_idx: None,
                        });
                    }
                }
                return Ok(None);
            }
            KeyCode::Down => {
                if !items.is_empty() {
                    let mut new_item_idx = if let Some(idx) = current_item_idx {
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
                    if active_submenu_idx.is_some() {
                        state.active_popup = Some(PopupType::Menu {
                            active_menu_idx,
                            active_item_idx,
                            active_submenu_idx,
                            active_submenu_item_idx: Some(new_item_idx),
                        });
                    } else {
                        state.active_popup = Some(PopupType::Menu {
                            active_menu_idx,
                            active_item_idx: Some(new_item_idx),
                            active_submenu_idx: None,
                            active_submenu_item_idx: None,
                        });
                    }
                }
                return Ok(None);
            }
            KeyCode::Enter => {
                if active_submenu_idx.is_some() {
                    if let Some(sub_item_idx) = active_submenu_item_idx {
                        state.active_popup = None;
                        let action = trigger_menu_item(
                            state,
                            context,
                            active_submenu_idx.unwrap(),
                            sub_item_idx,
                        );
                        return Ok(action);
                    }
                } else if let Some(idx) = active_item_idx {
                    if let Some(item) = items.get(idx) {
                        if let Some(sub_idx) = item.submenu_idx {
                            // Open submenu
                            state.active_popup = Some(PopupType::Menu {
                                active_menu_idx,
                                active_item_idx,
                                active_submenu_idx: Some(sub_idx),
                                active_submenu_item_idx: Some(0),
                            });
                            return Ok(None);
                        }
                    }
                    state.active_popup = None;
                    let action = trigger_menu_item(state, context, active_menu_idx, idx);
                    return Ok(action);
                } else {
                    state.active_popup = Some(PopupType::Menu {
                        active_menu_idx,
                        active_item_idx: Some(0),
                        active_submenu_idx: None,
                        active_submenu_item_idx: None,
                    });
                    return Ok(None);
                }
                return Ok(None);
            }
            KeyCode::Char(c) => {
                let lower_c = c.to_ascii_lowercase();

                // 1. Check dropdown items only if dropdown/submenu is open
                if current_item_idx.is_some() {
                    for (i, item) in items.iter().enumerate() {
                        if item.is_separator {
                            continue;
                        }
                        let parsed = crate::ui::hotkey::parse_hotkey(&item.label);
                        if let Some(hotkey) = parsed.hotkey {
                            if hotkey == lower_c {
                                if item.submenu_idx.is_some() {
                                    // Open submenu instead of triggering action
                                    state.active_popup = Some(PopupType::Menu {
                                        active_menu_idx,
                                        active_item_idx,
                                        active_submenu_idx: item.submenu_idx,
                                        active_submenu_item_idx: Some(0),
                                    });
                                    return Ok(None);
                                } else {
                                    state.active_popup = None;
                                    let action =
                                        trigger_menu_item(state, context, current_menu_idx, i);
                                    return Ok(action);
                                }
                            }
                        }
                    }
                }

                // 2. Check top menu titles if no submenu is open
                if active_submenu_idx.is_none() {
                    let titles = crate::ui::menu::get_menu_titles();
                    for (i, title) in titles.iter().enumerate() {
                        let parsed = crate::ui::hotkey::parse_hotkey(&title);
                        if let Some(hotkey) = parsed.hotkey {
                            if hotkey == lower_c {
                                state.active_popup = Some(PopupType::Menu {
                                    active_menu_idx: i,
                                    active_item_idx: Some(0),
                                    active_submenu_idx: None,
                                    active_submenu_item_idx: None,
                                });
                                return Ok(None);
                            }
                        }
                    }
                }
                return Ok(None);
            }
            _ => return Ok(None),
        }
    } else {
        Err(())
    }
}
