use super::types::MenuItemData;
use crate::app::state::{AppState, SortField};
use crate::config::localization::t;
use crate::keybindings::{Action, KeybindingResolver};

pub fn get_items(state: &AppState, resolver: &KeybindingResolver) -> Vec<MenuItemData> {
    let shortcut_for = |action: Action, fallback: &str| -> String {
        resolver
            .key_for_action(action)
            .map(|s| s.to_string())
            .unwrap_or_else(|| fallback.to_string())
    };

    vec![
        MenuItemData::new(
            t("menu_sort_name"),
            &shortcut_for(Action::SortByName, "Ctrl+F3"),
            state.left_panel.sort_field == SortField::Name,
        )
        .with_action(Action::SortByName),
        MenuItemData::new(
            t("menu_sort_ext"),
            &shortcut_for(Action::SortByExtension, "Ctrl+F4"),
            state.left_panel.sort_field == SortField::Extension,
        )
        .with_action(Action::SortByExtension),
        MenuItemData::new(
            t("menu_sort_write"),
            &shortcut_for(Action::SortByWriteTime, "Ctrl+F5"),
            state.left_panel.sort_field == SortField::Date,
        )
        .with_action(Action::SortByWriteTime),
        MenuItemData::new(
            t("menu_sort_size"),
            &shortcut_for(Action::SortBySize, "Ctrl+F6"),
            state.left_panel.sort_field == SortField::Size,
        )
        .with_action(Action::SortBySize),
        MenuItemData::new(
            t("menu_sort_unsorted"),
            &shortcut_for(Action::SortUnsorted, "Ctrl+F7"),
            state.left_panel.sort_field == SortField::Unsorted,
        )
        .with_action(Action::SortUnsorted),
        MenuItemData::new(
            t("menu_sort_create"),
            &shortcut_for(Action::SortByCreationTime, "Ctrl+F8"),
            false,
        )
        .with_action(Action::SortByCreationTime),
        MenuItemData::new(
            t("menu_sort_access"),
            &shortcut_for(Action::SortByAccessTime, "Ctrl+F9"),
            false,
        )
        .with_action(Action::SortByAccessTime),
        MenuItemData::new(
            t("menu_sort_desc"),
            &shortcut_for(Action::SortByDescription, "Ctrl+F10"),
            false,
        )
        .with_action(Action::SortByDescription),
        MenuItemData::new(
            t("menu_sort_owner"),
            &shortcut_for(Action::SortByOwner, "Ctrl+F11"),
            false,
        )
        .with_action(Action::SortByOwner),
    ]
}
