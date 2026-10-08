use super::types::{MenuBuilder, MenuItemData};
use crate::config::settings::Settings;
use crate::keybindings::{Action, KeybindingResolver};

pub fn get_items(resolver: &KeybindingResolver, settings: &Settings) -> Vec<MenuItemData> {
    let menu = MenuBuilder::new(resolver)
        .actions(&[
            ("menu_find_file", Action::FindFile, "Alt+F7"),
            ("menu_history", Action::CommandHistory, "Alt+F8"),
            ("menu_video_mode", Action::VideoMode, "Alt+F9"),
            ("menu_tree_view", Action::TreeView, "Alt+F10"),
            ("menu_file_view_hist", Action::FileViewHistory, "Alt+F11"),
            ("menu_folders_hist", Action::FoldersHistory, "Alt+F12"),
        ])
        .separator()
        .actions(&[
            ("menu_swap_panels", Action::SwapPanels, "Ctrl+U"),
            ("menu_panels_on_off", Action::ToggleBothPanels, "Ctrl+O"),
            ("menu_compare_folders", Action::CompareFolder, ""),
            ("menu_folder_sizes", Action::CalculateFolderSizes, ""),
            ("menu_disk_usage", Action::DiskUsage, ""),
        ])
        .separator()
        .actions(&[
            ("menu_user_menu", Action::UserMenu, "F2"),
            ("menu_edit_user_menu", Action::EditUserMenu, ""),
            ("menu_file_associations", Action::FileAssociations, ""),
            ("menu_folder_shortcuts", Action::FolderShortcutsConfig, ""),
            ("menu_hotlist", Action::Hotlist, "Ctrl+\\"),
            ("menu_file_panel_filter", Action::FilePanelFilter, "Ctrl+I"),
        ])
        .separator()
        .actions(&[
            ("menu_screens_list", Action::ScreensList, "F12"),
            ("menu_task_list", Action::TaskList, "Ctrl+W"),
        ])
        .plain("menu_hotplug_devices", "", None);
    if settings.plugins_developer_mode {
        menu.separator()
            .action((
                "menu_install_dev_plugin",
                Action::InstallDevPlugin,
                "Shift+F11",
            ))
            .build()
    } else {
        menu.build()
    }
}
