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
        MenuItemData::new(t("menu_view"), &shortcut_for(Action::View, "F3"), false)
            .with_action(Action::View),
        MenuItemData::new(
            t("menu_view_alt"),
            &shortcut_for(Action::ViewAlt, "Alt+F3"),
            false,
        )
        .with_action(Action::ViewAlt),
        MenuItemData::new(t("menu_edit"), &shortcut_for(Action::Edit, "F4"), false)
            .with_action(Action::Edit),
        MenuItemData::new(t("menu_copy"), &shortcut_for(Action::Copy, "F5"), false)
            .with_action(Action::Copy),
        MenuItemData::new(
            t("menu_print"),
            &shortcut_for(Action::PrintFile, "Alt+F5"),
            false,
        )
        .with_action(Action::PrintFile),
        MenuItemData::new(
            t("menu_rename_move"),
            &shortcut_for(Action::Move, "F6"),
            false,
        )
        .with_action(Action::Move),
        MenuItemData::new(
            t("menu_link"),
            &shortcut_for(Action::CreateLink, "Alt+F6"),
            false,
        )
        .with_action(Action::CreateLink),
        MenuItemData::new(
            t("menu_make_folder"),
            &shortcut_for(Action::MkDir, "F7"),
            false,
        )
        .with_action(Action::MkDir),
        MenuItemData::new(t("menu_delete"), &shortcut_for(Action::Delete, "F8"), false)
            .with_action(Action::Delete),
        MenuItemData::new(
            t("menu_wipe"),
            &shortcut_for(Action::WipeFile, "Alt+Del"),
            false,
        )
        .with_action(Action::WipeFile),
        MenuItemData::separator(),
        MenuItemData::new(
            t("menu_add_to_archive"),
            &shortcut_for(Action::CompressFiles, "Shf+F1"),
            false,
        )
        .with_action(Action::CompressFiles),
        MenuItemData::new(
            t("menu_extract_files"),
            &shortcut_for(Action::ExtractArchive, "Shf+F2"),
            false,
        )
        .with_action(Action::ExtractArchive),
        MenuItemData::new(
            t("menu_archive_commands"),
            &shortcut_for(Action::ArchiveCommands, "Shf+F3"),
            false,
        )
        .with_action(Action::ArchiveCommands),
        MenuItemData::separator(),
        MenuItemData::new(
            t("menu_file_attributes"),
            &shortcut_for(Action::FileAttributes, "Ctrl+A"),
            false,
        )
        .with_action(Action::FileAttributes),
        MenuItemData::new(
            t("menu_apply_command"),
            &shortcut_for(Action::ApplyCommand, "Ctrl+G"),
            false,
        )
        .with_action(Action::ApplyCommand),
        MenuItemData::new(
            t("menu_describe_files"),
            &shortcut_for(Action::DescribeFile, "Ctrl+Z"),
            false,
        )
        .with_action(Action::DescribeFile),
        MenuItemData::separator(),
        MenuItemData::new(
            t("menu_select_group"),
            &shortcut_for(Action::SelectGroup, "Gray+"),
            false,
        )
        .with_action(Action::SelectGroup),
        MenuItemData::new(
            t("menu_unselect_group"),
            &shortcut_for(Action::UnselectGroup, "Gray-"),
            false,
        )
        .with_action(Action::UnselectGroup),
        MenuItemData::new(
            t("menu_invert_selection"),
            &shortcut_for(Action::InvertSelection, "Gray*"),
            false,
        )
        .with_action(Action::InvertSelection),
        MenuItemData::new(
            t("menu_restore_selection"),
            &shortcut_for(Action::RestoreSelection, "Ctrl+M"),
            false,
        )
        .with_action(Action::RestoreSelection),
        MenuItemData::separator(),
        MenuItemData::new(t("menu_exit"), &shortcut_for(Action::Quit, "F10"), false)
            .with_action(Action::Quit),
    ]
}
