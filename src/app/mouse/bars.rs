//! Mouse on the bars around the panels: the top menu (titles, drop-downs,
//! submenus) and the function-key bar.

use super::Click;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::terminal::{Event, TerminalBackend};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton};

/// Open menu: top menu, focused item, open submenu and its focused item.
type MenuFocus = (usize, Option<usize>, Option<usize>, Option<usize>);

fn menu_focus(state: &AppState) -> Option<MenuFocus> {
    match state.dialogs.top() {
        Some(&PopupType::Menu {
            active_menu_idx,
            active_item_idx,
            active_submenu_idx,
            active_submenu_item_idx,
        }) => Some((
            active_menu_idx,
            active_item_idx,
            active_submenu_idx,
            active_submenu_item_idx,
        )),
        _ => None,
    }
}

/// Opens (or refocuses) the menu; returns whether anything changed.
fn set_menu(state: &mut AppState, focus: MenuFocus) -> bool {
    if menu_focus(state) == Some(focus) {
        return false;
    }
    let (menu, item, submenu, sub_item) = focus;
    let popup = PopupType::Menu {
        active_menu_idx: menu,
        active_item_idx: item,
        active_submenu_idx: submenu,
        active_submenu_item_idx: sub_item,
    };
    if menu_focus(state).is_some() {
        state.dialogs.replace(popup);
    } else {
        state.dialogs.push(popup);
    }
    true
}

/// `true` when a click on the menu bar should act although a popup is open:
/// only the menu itself may be open.
pub fn menu_bar_reachable(state: &AppState) -> bool {
    state.dialogs.is_none() || menu_focus(state).is_some()
}

/// Top menu whose title covers column `x` of the (hidden) bar.
fn title_at(x: u16, width: u16) -> Option<usize> {
    crate::ui::menu::title_cells(ratatui::layout::Rect::new(0, 0, width, 1))
        .iter()
        .position(|cell| x >= cell.x && x < cell.right())
}

/// A click on the top row while the menu bar is hidden opens the menu
/// under the pointer.
pub fn click_reveal(state: &mut AppState, click: Click, width: u16) -> bool {
    match title_at(click.column, width) {
        Some(menu) => click_title(state, menu, click),
        None => false,
    }
}

/// A click on top menu `menu`: opens its drop-down, or closes it when it is
/// the one already open.
pub fn click_title(state: &mut AppState, menu: usize, click: Click) -> bool {
    if click.button != MouseButton::Left {
        return false;
    }
    match menu_focus(state) {
        Some((open, Some(_), _, _)) if open == menu => {
            state.dialogs.clear();
            true
        }
        _ => set_menu(state, (menu, Some(0), None, None)),
    }
}

/// A click on row `row` of drop-down `menu`: opens the row's submenu or
/// runs its command (as Enter would).
pub async fn click_item(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    (menu, row): (usize, usize),
    click: Click,
) -> anyhow::Result<bool> {
    if click.button != MouseButton::Left {
        return Ok(false);
    }
    let Some((top, ..)) = menu_focus(state) else {
        return Ok(false);
    };
    let items =
        crate::ui::menu::get_menu_items(menu, state, &context.resolver, &context.config.settings);
    let Some(entry) = items.get(row).filter(|entry| !entry.is_separator) else {
        return Ok(false);
    };
    if let Some(sub) = entry.submenu_idx {
        return Ok(set_menu(state, (top, Some(row), Some(sub), Some(0))));
    }
    state.dialogs.clear();
    if let Some(action) = crate::app::menu_handler::trigger_menu_item(state, context, menu, row) {
        super::act(state, context, terminal_backend, action).await?;
    }
    Ok(true)
}

/// Pointer motion over the open menu: titles switch drop-downs, rows take
/// the highlight and open their submenus.
pub fn hover(
    state: &mut AppState,
    context: &AppContext,
    target: &crate::ui::hit::HitTarget,
) -> bool {
    use crate::ui::hit::HitTarget;
    if *target == HitTarget::MenuReveal {
        if menu_focus(state).is_none() {
            state.mouse.menu_revealed = true;
            return set_menu(state, (0, None, None, None));
        }
        return false;
    }
    let Some((top, item, submenu, _)) = menu_focus(state) else {
        return false;
    };
    // A bar shown by pointing hides again when the pointer leaves it
    // without opening a drop-down.
    if state.mouse.menu_revealed && item.is_none() && !matches!(target, HitTarget::MenuTitle(_)) {
        state.mouse.menu_revealed = false;
        state.dialogs.clear();
        return true;
    }
    match *target {
        HitTarget::MenuTitle(menu) if item.is_some() && menu != top => {
            set_menu(state, (menu, Some(0), None, None))
        }
        HitTarget::MenuItem { menu, row } if menu == top => {
            let items = crate::ui::menu::get_menu_items(
                menu,
                state,
                &context.resolver,
                &context.config.settings,
            );
            let sub = items.get(row).and_then(|entry| entry.submenu_idx);
            set_menu(state, (top, Some(row), sub, sub.map(|_| 0)))
        }
        HitTarget::MenuItem { menu, row } if Some(menu) == submenu => {
            set_menu(state, (top, item, submenu, Some(row)))
        }
        _ => false,
    }
}

/// A click on `F<number>` presses that key with the modifiers whose row the
/// bar showed, so every screen (panels, editor, viewer) handles it as usual.
pub async fn click_fkey(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    (number, modifiers): (u8, KeyModifiers),
    click: Click,
) -> anyhow::Result<bool> {
    if click.button != MouseButton::Left {
        return Ok(false);
    }
    let key = KeyEvent::new(KeyCode::F(number), modifiers);
    Box::pin(crate::app::app::events::handle_input_event(
        state,
        context,
        terminal_backend,
        Event::Key(key),
    ))
    .await?;
    Ok(true)
}
