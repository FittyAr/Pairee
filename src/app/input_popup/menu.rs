//! Top menu bar: drop-down navigation, submenus and hotkeys.

use crate::app::context::AppContext;
use crate::app::list_nav::{wrap_next, wrap_prev};
use crate::app::menu_handler::trigger_menu_item;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crate::ui::menu::MenuItemData;
use crossterm::event::{KeyCode, KeyEvent};

/// Number of top-level menus (Left, Files, Commands, Options, Right).
const TOP_MENUS: usize = 5;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(&PopupType::Menu {
        active_menu_idx: menu,
        active_item_idx: item,
        active_submenu_idx: submenu,
        active_submenu_item_idx: sub_item,
    }) = state.dialogs.top()
    else {
        return Err(());
    };
    let current_menu = submenu.unwrap_or(menu);
    let items = crate::ui::menu::get_menu_items(
        current_menu,
        state,
        &context.resolver,
        &context.config.settings,
    );

    // The new menu state, or an item to run.
    let next = match key.code {
        KeyCode::Esc if submenu.is_none() => {
            state.dialogs.clear();
            return Ok(None);
        }
        KeyCode::Esc | KeyCode::Left if submenu.is_some() => (menu, item, None, None),
        KeyCode::Left => (wrap_prev(menu, TOP_MENUS), item.map(|_| 0), None, None),
        KeyCode::Right if submenu.is_some() => return Ok(None),
        KeyCode::Right => match item
            .and_then(|i| items.get(i))
            .and_then(|it| it.submenu_idx)
        {
            Some(sub) => (menu, item, Some(sub), Some(0)),
            None => (wrap_next(menu, TOP_MENUS), item.map(|_| 0), None, None),
        },
        KeyCode::Up | KeyCode::Down if items.is_empty() => return Ok(None),
        KeyCode::Up | KeyCode::Down => {
            let row = step_item(&items, sub_item.or(item), key.code == KeyCode::Up);
            match submenu {
                Some(_) => (menu, item, submenu, Some(row)),
                None => (menu, Some(row), None, None),
            }
        }
        KeyCode::Enter => match (submenu, sub_item, item) {
            (Some(sub), Some(row), _) => return run_item(state, context, sub, row),
            (Some(_), None, _) => return Ok(None),
            (None, _, Some(row)) => match items.get(row).and_then(|it| it.submenu_idx) {
                Some(sub) => (menu, item, Some(sub), Some(0)),
                None => return run_item(state, context, menu, row),
            },
            (None, _, None) => (menu, Some(0), None, None),
        },
        KeyCode::Char(c) => {
            let c = c.to_ascii_lowercase();
            // Items of the open drop-down / submenu first.
            let hit = (sub_item.or(item).is_some())
                .then(|| {
                    items
                        .iter()
                        .enumerate()
                        .find(|(_, it)| !it.is_separator && hotkey(&it.label) == Some(c))
                })
                .flatten();
            match hit {
                Some((_, it)) if it.submenu_idx.is_some() => (menu, item, it.submenu_idx, Some(0)),
                Some((row, _)) => return run_item(state, context, current_menu, row),
                // Then the top menu titles (when no submenu is open).
                None => match crate::ui::menu::get_menu_titles()
                    .iter()
                    .position(|title| submenu.is_none() && hotkey(title) == Some(c))
                {
                    Some(top) => (top, Some(0), None, None),
                    None => return Ok(None),
                },
            }
        }
        _ => return Ok(None),
    };
    let (menu, item, submenu, sub_item) = next;
    state.dialogs.replace(PopupType::Menu {
        active_menu_idx: menu,
        active_item_idx: item,
        active_submenu_idx: submenu,
        active_submenu_item_idx: sub_item,
    });
    Ok(None)
}

/// Closes the menu and runs item `row` of menu `menu_idx`.
fn run_item(
    state: &mut AppState,
    context: &mut AppContext,
    menu_idx: usize,
    row: usize,
) -> Result<Option<Action>, ()> {
    state.dialogs.clear();
    Ok(trigger_menu_item(state, context, menu_idx, row))
}

/// Previous / next item from `current`, wrapping and skipping separators.
/// Without a current item, Up starts at the last item and Down at the first.
fn step_item(items: &[MenuItemData], current: Option<usize>, up: bool) -> usize {
    let len = items.len();
    let step = if up { wrap_prev } else { wrap_next };
    let mut row = match current {
        Some(idx) => step(idx, len),
        None if up => len - 1,
        None => 0,
    };
    while items[row].is_separator {
        row = step(row, len);
    }
    row
}

fn hotkey(label: &str) -> Option<char> {
    crate::ui::hotkey::parse_hotkey(label).hotkey
}
