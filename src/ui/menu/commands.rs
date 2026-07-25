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
        MenuItemData::new(
            t("menu_find_file"),
            &shortcut_for(Action::FindFile, "Alt+F7"),
            false,
        )
        .with_action(Action::FindFile),
        MenuItemData::new(
            t("menu_history"),
            &shortcut_for(Action::CommandHistory, "Alt+F8"),
            false,
        )
        .with_action(Action::CommandHistory),
        MenuItemData::new(
            t("menu_video_mode"),
            &shortcut_for(Action::VideoMode, "Alt+F9"),
            false,
        )
        .with_action(Action::VideoMode),
        MenuItemData::new(
            t("menu_tree_view"),
            &shortcut_for(Action::TreeView, "Alt+F10"),
            false,
        )
        .with_action(Action::TreeView),
        MenuItemData::new(
            t("menu_file_view_hist"),
            &shortcut_for(Action::FileViewHistory, "Alt+F11"),
            false,
        )
        .with_action(Action::FileViewHistory),
        MenuItemData::new(
            t("menu_folders_hist"),
            &shortcut_for(Action::FoldersHistory, "Alt+F12"),
            false,
        )
        .with_action(Action::FoldersHistory),
        MenuItemData::separator(),
        MenuItemData::new(
            t("menu_swap_panels"),
            &shortcut_for(Action::SwapPanels, "Ctrl+U"),
            false,
        )
        .with_action(Action::SwapPanels),
        MenuItemData::new(
            t("menu_panels_on_off"),
            &shortcut_for(Action::ToggleBothPanels, "Ctrl+O"),
            false,
        )
        .with_action(Action::ToggleBothPanels),
        MenuItemData::new(
            t("menu_compare_folders"),
            &shortcut_for(Action::CompareFolder, ""),
            false,
        )
        .with_action(Action::CompareFolder),
        MenuItemData::separator(),
        MenuItemData::new(
            t("menu_user_menu"),
            &shortcut_for(Action::UserMenu, "F2"),
            false,
        )
        .with_action(Action::UserMenu),
        MenuItemData::new(
            t("menu_edit_user_menu"),
            &shortcut_for(Action::EditUserMenu, ""),
            false,
        )
        .with_action(Action::EditUserMenu),
        MenuItemData::new(
            t("menu_file_associations"),
            &shortcut_for(Action::FileAssociations, ""),
            false,
        )
        .with_action(Action::FileAssociations),
        MenuItemData::new(
            t("menu_folder_shortcuts"),
            &shortcut_for(Action::FolderShortcutsConfig, ""),
            false,
        )
        .with_action(Action::FolderShortcutsConfig),
        MenuItemData::new(
            t("menu_file_panel_filter"),
            &shortcut_for(Action::FilePanelFilter, "Ctrl+I"),
            false,
        )
        .with_action(Action::FilePanelFilter),
        MenuItemData::separator(),
        MenuItemData::new(
            t("menu_screens_list"),
            &shortcut_for(Action::ScreensList, "F12"),
            false,
        )
        .with_action(Action::ScreensList),
        MenuItemData::new(
            t("menu_task_list"),
            &shortcut_for(Action::TaskList, "Ctrl+W"),
            false,
        )
        .with_action(Action::TaskList),
        MenuItemData::new(t("menu_hotplug_devices"), "", false),
    ];

    if settings.plugins_developer_mode {
        items.push(MenuItemData::separator());
        items.push(
            MenuItemData::new(
                t("menu_install_dev_plugin"),
                &shortcut_for(Action::InstallDevPlugin, "Shift+F11"),
                false,
            )
            .with_action(Action::InstallDevPlugin),
        );
    }

    items
}
