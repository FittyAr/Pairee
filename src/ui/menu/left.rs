use super::types::MenuItemData;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::{Action, KeybindingResolver};
use crate::config::settings::Settings;

pub fn get_items(
    state: &AppState,
    resolver: &KeybindingResolver,
    settings: &Settings,
) -> Vec<MenuItemData> {
    let shortcut_for = |action: Action, fallback: &str| -> String {
        resolver
            .key_for_action(action)
            .map(|s| s.to_string())
            .unwrap_or_else(|| fallback.to_string())
    };

    let mut items = vec![
        MenuItemData::new(format!("{} >", t("menu_view_mode")), "", false).with_submenu(5),
        MenuItemData::separator(),
        MenuItemData::new(
            t("menu_info_panel"),
            &shortcut_for(Action::InfoPanel, "Ctrl+L"),
            matches!(state.active_popup, Some(PopupType::InfoPanel { .. })),
        )
        .with_action(Action::InfoPanel),
        MenuItemData::new(
            t("menu_quick_view"),
            &shortcut_for(Action::QuickView, "Ctrl+Q"),
            state.quick_view_active,
        )
        .with_action(Action::QuickView),
        MenuItemData::separator(),
        MenuItemData::new(
            t("menu_sort_modes"),
            &shortcut_for(Action::SortModes, "Ctrl+F12"),
            false,
        )
        .with_action(Action::SortModes),
        MenuItemData::new(format!("{} >", t("menu_sort_by")), "", false).with_submenu(6),
        MenuItemData::new(
            t("menu_show_long_names"),
            &shortcut_for(Action::ToggleLongNames, "Ctrl+N"),
            state.left_panel.show_long_names,
        )
        .with_action(Action::ToggleLongNames),
        MenuItemData::new(
            t("menu_panel_on_off"),
            &shortcut_for(Action::TogglePanelLeft, "Ctrl+F1"),
            state.left_panel_visible,
        )
        .with_action(Action::TogglePanelLeft),
        MenuItemData::new(
            t("menu_re_read"),
            &shortcut_for(Action::RereadPanel, "Ctrl+R"),
            false,
        )
        .with_action(Action::RereadPanel),
        MenuItemData::new(
            t("menu_change_drive"),
            &shortcut_for(Action::DriveSelectLeft, "Alt+F1"),
            false,
        )
        .with_action(Action::DriveSelectLeft),
        MenuItemData::new(
            t("menu_connect_ssh"),
            &shortcut_for(Action::SshConnect, "Ctrl+Shift+S"),
            false,
        )
        .with_action(Action::SshConnect),
    ];

    if state.left_panel.ssh_conn.is_some() {
        items.push(
            MenuItemData::new(
                t("menu_disconnect_ssh"),
                &shortcut_for(Action::SshDisconnect, ""),
                false,
            )
            .with_action(Action::SshDisconnect),
        );
    }

    if settings.git_enabled {
        if crate::git::repo::find_repo(&state.left_panel.current_path).is_some() {
            items.push(MenuItemData::separator());
            items.push(
                MenuItemData::new(
                    t("menu_git"),
                    &shortcut_for(Action::OpenGitPanel, "Alt+G"),
                    false,
                )
                .with_action(Action::OpenGitPanel),
            );
        }
    }

    items
}
