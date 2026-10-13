//! The menu bar (shown or hidden), its drop-downs and the function keys.

use super::{panels, top};
use crate::app::state::popup::MkDirPrompt;
use crate::app::state::{ActivePanel, PopupType};
use crate::test_harness::Harness;
use crate::ui::hit::HitTarget;

/// Row of the first item of menu `menu` that `pick` accepts.
fn menu_row(
    h: &Harness,
    menu: usize,
    pick: impl Fn(&crate::ui::menu::MenuItemData) -> bool,
) -> usize {
    crate::ui::menu::get_menu_items(menu, &h.state, &h.ctx.resolver, &h.ctx.config.settings)
        .iter()
        .position(pick)
        .expect("menu row")
}

/// Points at the hidden menu bar so its titles show.
fn reveal_menu(h: &mut Harness) {
    h.hover(1, 0);
}

fn open_menu(h: &Harness) -> Option<(usize, Option<usize>, Option<usize>)> {
    match top(h) {
        Some(&PopupType::Menu {
            active_menu_idx,
            active_item_idx,
            active_submenu_idx,
            ..
        }) => Some((active_menu_idx, active_item_idx, active_submenu_idx)),
        _ => None,
    }
}

#[test]
fn menu_titles_open_and_close_their_drop_downs() {
    let mut h = Harness::new();
    reveal_menu(&mut h);
    let (x, y) = h.hit_cell(|t| *t == HitTarget::MenuTitle(1));
    h.click(x + 1, y);
    assert_eq!(open_menu(&h), Some((1, Some(0), None)));
    // The drop-down opens under its own title.
    let (item_x, _) = h.hit_cell(|t| matches!(t, HitTarget::MenuItem { menu: 1, .. }));
    assert!(item_x > x, "drop-down at {item_x}, title at {x}");
    // Another title switches menus; the open one closes it.
    let (x2, _) = h.hit_cell(|t| *t == HitTarget::MenuTitle(3));
    h.click(x2 + 1, y);
    assert_eq!(open_menu(&h).map(|m| m.0), Some(3));
    h.click(x2 + 1, y);
    assert!(top(&h).is_none());
}

#[test]
fn clicking_a_menu_item_runs_it() {
    let mut h = Harness::new();
    reveal_menu(&mut h);
    let (x, y) = h.hit_cell(|t| *t == HitTarget::MenuTitle(1));
    h.click(x + 1, y);
    let row = menu_row(&h, 1, |item| {
        item.action == Some(crate::keybindings::Action::MkDir)
    });
    let (item_x, item_y) = h.hit_cell(|t| *t == HitTarget::MenuItem { menu: 1, row });
    h.click(item_x + 2, item_y);
    assert!(
        matches!(top(&h), Some(PopupType::MkDirPrompt(MkDirPrompt { .. }))),
        "{}",
        h.screen_text()
    );
}

#[test]
fn hovering_moves_the_menu_highlight_and_opens_submenus() {
    let mut h = Harness::new();
    reveal_menu(&mut h);
    let (x, y) = h.hit_cell(|t| *t == HitTarget::MenuTitle(0));
    h.click(x + 1, y);
    let row = menu_row(&h, 0, |item| item.submenu_idx.is_some());
    let (item_x, item_y) = h.hit_cell(|t| *t == HitTarget::MenuItem { menu: 0, row });
    h.hover(item_x + 2, item_y);
    let (menu, item, submenu) = open_menu(&h).expect("menu open");
    assert_eq!((menu, item), (0, Some(row)));
    let sub = submenu.expect("submenu opened on hover");
    // Hovering another title switches the drop-down.
    let (x2, _) = h.hit_cell(|t| *t == HitTarget::MenuTitle(4));
    h.hover(x2 + 1, y);
    assert_eq!(open_menu(&h), Some((4, Some(0), None)));
    let _ = sub;
}

#[test]
fn clicking_outside_the_menu_closes_it() {
    let mut h = panels(0);
    reveal_menu(&mut h);
    let (x, y) = h.hit_cell(|t| *t == HitTarget::MenuTitle(2));
    h.click(x + 1, y);
    assert!(open_menu(&h).is_some());
    // A blank row of the right panel, clear of the drop-down.
    h.click(90, 20);
    assert!(top(&h).is_none());
    // The click only closed the menu.
    assert_eq!(h.state.panels.active, ActivePanel::Left);
}

#[test]
fn function_keys_on_the_bar_press_their_key() {
    let mut h = Harness::new();
    let (x, y) = h.hit_cell(|t| {
        matches!(
            t,
            HitTarget::FKey {
                number: 7,
                modifiers
            } if modifiers.is_empty()
        )
    });
    h.click(x + 1, y);
    assert!(
        matches!(top(&h), Some(PopupType::MkDirPrompt(MkDirPrompt { .. }))),
        "{}",
        h.screen_text()
    );
}

#[test]
fn the_hidden_menu_bar_shows_while_pointed_at() {
    let mut h = panels(0);
    h.hover(3, 0);
    assert_eq!(open_menu(&h), Some((0, None, None)));
    h.hover(3, 6);
    assert!(top(&h).is_none(), "leaving the bar hides it again");
    // A click on the top row opens the menu under the pointer.
    let (x, _) = {
        h.hover(3, 0);
        h.hit_cell(|t| *t == HitTarget::MenuTitle(1))
    };
    h.hover(3, 6);
    h.click(x + 1, 0);
    assert_eq!(open_menu(&h), Some((1, Some(0), None)));
}
