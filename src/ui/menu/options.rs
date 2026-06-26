use super::types::MenuItemData;
use crate::config::localization::t;
use crate::keybindings::{Action, KeybindingResolver};

pub fn get_items(resolver: &KeybindingResolver) -> Vec<MenuItemData> {
    let shortcut_for = |action: Action, fallback: &str| -> String {
        resolver
            .key_for_action(action)
            .map(|s| s.to_string())
            .unwrap_or_else(|| fallback.to_string())
    };

    vec![
        MenuItemData::new(t("menu_help"), &shortcut_for(Action::Help, "F1"), false)
            .with_action(Action::Help),
        MenuItemData::new(t("menu_about"), &shortcut_for(Action::About, ""), false)
            .with_action(Action::About),
        MenuItemData::separator(),
        MenuItemData::new(
            t("menu_configuration"),
            &shortcut_for(Action::SystemSettings, ""),
            false,
        )
        .with_action(Action::SystemSettings),
        MenuItemData::new(t("menu_check_updates"), "", false).with_action(Action::CheckForUpdates),
        MenuItemData::separator(),
        MenuItemData::new(
            t("menu_save_setup"),
            &shortcut_for(Action::SaveSetup, "Shf+F9"),
            false,
        )
        .with_action(Action::SaveSetup),
    ]
}
