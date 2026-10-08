//! "View mode" and "Sort by" submenus of the Left / Right menus.

use super::types::{MenuBuilder, MenuItemData};
use crate::app::state::{ActivePanel, AppState, PanelViewMode, SortField};
use crate::keybindings::{Action, KeybindingResolver};

const VIEW_MODES: [(&str, Action, &str, PanelViewMode); 9] = [
    (
        "menu_brief",
        Action::PanelViewBrief,
        "Ctrl+1",
        PanelViewMode::Brief,
    ),
    (
        "menu_medium",
        Action::PanelViewMedium,
        "Ctrl+2",
        PanelViewMode::Medium,
    ),
    (
        "menu_full",
        Action::PanelViewFull,
        "Ctrl+3",
        PanelViewMode::Full,
    ),
    (
        "menu_wide",
        Action::PanelViewWide,
        "Ctrl+4",
        PanelViewMode::Wide,
    ),
    (
        "menu_detailed",
        Action::PanelViewDetailed,
        "Ctrl+5",
        PanelViewMode::Detailed,
    ),
    (
        "menu_descriptions",
        Action::PanelViewDescriptions,
        "Ctrl+6",
        PanelViewMode::Descriptions,
    ),
    (
        "menu_file_owners",
        Action::PanelViewFileOwners,
        "Ctrl+7",
        PanelViewMode::FileOwners,
    ),
    (
        "menu_file_links",
        Action::PanelViewFileLinks,
        "Ctrl+8",
        PanelViewMode::FileLinks,
    ),
    (
        "menu_alt_full",
        Action::PanelViewAltFull,
        "Ctrl+9",
        PanelViewMode::AltFull,
    ),
];

/// Sort entries; `None` marks orders that are never shown as current.
const SORT_FIELDS: [(&str, Action, &str, Option<SortField>); 9] = [
    (
        "menu_sort_name",
        Action::SortByName,
        "Ctrl+F3",
        Some(SortField::Name),
    ),
    (
        "menu_sort_ext",
        Action::SortByExtension,
        "Ctrl+F4",
        Some(SortField::Extension),
    ),
    (
        "menu_sort_write",
        Action::SortByWriteTime,
        "Ctrl+F5",
        Some(SortField::Date),
    ),
    (
        "menu_sort_size",
        Action::SortBySize,
        "Ctrl+F6",
        Some(SortField::Size),
    ),
    (
        "menu_sort_unsorted",
        Action::SortUnsorted,
        "Ctrl+F7",
        Some(SortField::Unsorted),
    ),
    (
        "menu_sort_create",
        Action::SortByCreationTime,
        "Ctrl+F8",
        None,
    ),
    (
        "menu_sort_access",
        Action::SortByAccessTime,
        "Ctrl+F9",
        None,
    ),
    (
        "menu_sort_desc",
        Action::SortByDescription,
        "Ctrl+F10",
        None,
    ),
    ("menu_sort_owner", Action::SortByOwner, "Ctrl+F11", None),
];

pub fn view_items(
    state: &AppState,
    resolver: &KeybindingResolver,
    panel: ActivePanel,
) -> Vec<MenuItemData> {
    let current = state.panels.side(panel).view_mode;
    VIEW_MODES
        .iter()
        .fold(
            MenuBuilder::new(resolver),
            |menu, &(label, action, keys, mode)| {
                menu.toggle((label, action, keys), current == mode)
            },
        )
        .build()
}

pub fn sort_items(
    state: &AppState,
    resolver: &KeybindingResolver,
    panel: ActivePanel,
) -> Vec<MenuItemData> {
    let current = state.panels.side(panel).sort_field;
    SORT_FIELDS
        .iter()
        .fold(
            MenuBuilder::new(resolver),
            |menu, &(label, action, keys, field)| {
                menu.toggle((label, action, keys), field == Some(current))
            },
        )
        .build()
}
