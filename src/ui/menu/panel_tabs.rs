//! "Tabs" submenu of the Left / Right menus.

use super::types::{ActionRow, MenuBuilder, MenuItemData};
use crate::app::state::{ActivePanel, AppState};
use crate::keybindings::{Action, KeybindingResolver};

const TAB_ROWS: [ActionRow; 8] = [
    ("menu_tab_new", Action::NewTab, "Alt+T"),
    ("menu_tab_open_cursor", Action::OpenInNewTab, "Alt+O"),
    ("menu_tab_close", Action::CloseTab, "Alt+W"),
    ("menu_tab_next", Action::NextTab, "Alt+PgDn"),
    ("menu_tab_prev", Action::PrevTab, "Alt+PgUp"),
    ("menu_tab_move_left", Action::MoveTabLeft, "Alt+Shift+PgUp"),
    (
        "menu_tab_move_right",
        Action::MoveTabRight,
        "Alt+Shift+PgDn",
    ),
    ("menu_tab_rename", Action::RenameTab, ""),
];

pub fn tab_items(
    state: &AppState,
    resolver: &KeybindingResolver,
    panel: ActivePanel,
) -> Vec<MenuItemData> {
    let locked = state.panels.tabs(panel).active().lock.is_some();
    MenuBuilder::new(resolver)
        .actions(&TAB_ROWS)
        .toggle(("menu_tab_lock", Action::ToggleTabLock, ""), locked)
        .build()
}
