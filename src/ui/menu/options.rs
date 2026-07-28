use super::types::MenuItemData;
use crate::config::localization::t;
use crate::keybindings::{Action, KeybindingResolver};

pub fn get_items(
    resolver: &KeybindingResolver,
    settings: &crate::config::settings::Settings,
) -> Vec<MenuItemData> {
    let shortcut_for = |action: Action, fallback: &str| -> String {
        resolver
            .key_for_action(action)
            .map(|s| s.to_string())
            .unwrap_or_else(|| fallback.to_string())
    };

    let mut items = vec![
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
        MenuItemData::separator(),
        MenuItemData::new(
            t("menu_plugin_commands"),
            &shortcut_for(Action::PluginMenu, ""),
            false,
        )
        .with_action(Action::PluginMenu),
    ];

    // The "Install development plugin" entry is only meaningful when
    // the user has opted into the developer mode (a setting that
    // gates every Dev Tools surface in the popup). Keeping the
    // visibility check here matches the previous behaviour from the
    // Commands menu.
    if settings.plugins_developer_mode {
        items.push(
            MenuItemData::new(
                t("menu_install_dev_plugin"),
                &shortcut_for(Action::InstallDevPlugin, "Shift+F11"),
                false,
            )
            .with_action(Action::InstallDevPlugin),
        );
    }

    items.push(
        MenuItemData::new(t("menu_exit"), &shortcut_for(Action::Quit, "F10"), false)
            .with_action(Action::Quit),
    );

    items
}
