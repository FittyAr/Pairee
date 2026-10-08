use super::types::{MenuBuilder, MenuItemData};
use crate::config::settings::Settings;
use crate::keybindings::{Action, KeybindingResolver};

pub fn get_items(resolver: &KeybindingResolver, settings: &Settings) -> Vec<MenuItemData> {
    MenuBuilder::new(resolver)
        .actions(&[
            ("menu_view", Action::View, "F3"),
            ("menu_view_alt", Action::ViewAlt, "Alt+F3"),
            ("menu_edit", Action::Edit, "F4"),
            ("menu_copy", Action::Copy, "F5"),
            ("menu_copy_path", Action::CopyPath, "Ctrl+Shift+C"),
            ("menu_print", Action::PrintFile, "Alt+F5"),
            ("menu_rename_move", Action::Move, "F6"),
            ("menu_rename", Action::Rename, "F7"),
            ("menu_multi_rename", Action::MultiRename, "Shf+F6"),
            ("menu_link", Action::CreateLink, "Alt+F6"),
            ("menu_make_folder", Action::MkDir, ""),
            ("menu_delete", Action::Delete, "F8"),
            ("menu_wipe", Action::WipeFile, "Alt+Del"),
        ])
        .separator()
        .actions(&[
            ("menu_add_to_archive", Action::CompressFiles, "Shf+F1"),
            ("menu_extract_files", Action::ExtractArchive, "Shf+F2"),
            ("menu_archive_commands", Action::ArchiveCommands, "Shf+F3"),
        ])
        .separator()
        .actions(&[
            ("menu_file_attributes", Action::FileAttributes, "Ctrl+A"),
            ("menu_apply_command", Action::ApplyCommand, "Ctrl+G"),
            ("menu_describe_files", Action::DescribeFile, "Ctrl+Z"),
        ])
        .separator()
        .actions(&[
            ("menu_select_group", Action::SelectGroup, "Gray+"),
            ("menu_unselect_group", Action::UnselectGroup, "Gray-"),
            ("menu_invert_selection", Action::InvertSelection, "Gray*"),
            ("menu_restore_selection", Action::RestoreSelection, "Ctrl+M"),
        ])
        .separator()
        .actions_if(
            settings.plugins_enabled,
            &[("menu_plugin_commands", Action::PluginMenu, "")],
        )
        .action(("menu_exit", Action::Quit, "F10"))
        .build()
}
