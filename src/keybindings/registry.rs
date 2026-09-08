//! Canonical action catalogue (id + `Action` + palette flag + category).
//!
//! Chord strings stay in `keymaps/*.toml`. This is **not** a second keymap.

use super::actions::Action;

/// Grouping for palette / future help surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionCategory {
    Navigation,
    Files,
    View,
    Search,
    Git,
    System,
}

/// One logical action as the rest of the UI should describe it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionDef {
    pub id: &'static str,
    pub action: Action,
    pub in_palette: bool,
    pub category: ActionCategory,
}

impl ActionDef {
    /// Human palette label (`copy_path` → `copy path`).
    pub fn palette_label(self) -> String {
        self.id.replace('_', " ")
    }
}

const fn d(
    id: &'static str,
    action: Action,
    in_palette: bool,
    category: ActionCategory,
) -> ActionDef {
    ActionDef {
        id,
        action,
        in_palette,
        category,
    }
}

/// Shipped catalogue. Palette entries match the previous hardcoded name list.
pub const CATALOG: &[ActionDef] = &[
    d("move_up", Action::MoveUp, true, ActionCategory::Navigation),
    d(
        "move_down",
        Action::MoveDown,
        true,
        ActionCategory::Navigation,
    ),
    d(
        "change_panel",
        Action::ChangePanel,
        true,
        ActionCategory::Navigation,
    ),
    d("help", Action::Help, true, ActionCategory::System),
    d("about", Action::About, true, ActionCategory::System),
    d("copy", Action::Copy, true, ActionCategory::Files),
    d("copy_path", Action::CopyPath, true, ActionCategory::Files),
    d("move", Action::Move, true, ActionCategory::Files),
    d("rename", Action::Rename, true, ActionCategory::Files),
    d("delete", Action::Delete, true, ActionCategory::Files),
    d("mkdir", Action::MkDir, true, ActionCategory::Files),
    d("view", Action::View, true, ActionCategory::Files),
    d("edit", Action::Edit, true, ActionCategory::Files),
    d("find_file", Action::FindFile, true, ActionCategory::Search),
    d("refresh", Action::Refresh, true, ActionCategory::View),
    d(
        "toggle_hidden",
        Action::ToggleHidden,
        true,
        ActionCategory::View,
    ),
    d(
        "swap_panels",
        Action::SwapPanels,
        true,
        ActionCategory::View,
    ),
    d(
        "open_git_panel",
        Action::OpenGitPanel,
        true,
        ActionCategory::Git,
    ),
    d(
        "ssh_connect",
        Action::SshConnect,
        true,
        ActionCategory::System,
    ),
    d(
        "ssh_disconnect",
        Action::SshDisconnect,
        true,
        ActionCategory::System,
    ),
    d(
        "plugin_menu",
        Action::PluginMenu,
        true,
        ActionCategory::System,
    ),
    d(
        "system_settings",
        Action::SystemSettings,
        true,
        ActionCategory::System,
    ),
    d(
        "check_for_updates",
        Action::CheckForUpdates,
        true,
        ActionCategory::System,
    ),
    d(
        "toggle_transfer_panel",
        Action::ToggleTransferPanel,
        true,
        ActionCategory::System,
    ),
    d("quit", Action::Quit, true, ActionCategory::System),
    d(
        "compare_folder",
        Action::CompareFolder,
        true,
        ActionCategory::Files,
    ),
    d("task_list", Action::TaskList, true, ActionCategory::System),
    d("tree_view", Action::TreeView, true, ActionCategory::View),
    d(
        "command_history",
        Action::CommandHistory,
        true,
        ActionCategory::Search,
    ),
    d(
        "folders_history",
        Action::FoldersHistory,
        true,
        ActionCategory::Search,
    ),
    d(
        "file_view_history",
        Action::FileViewHistory,
        true,
        ActionCategory::Search,
    ),
    d(
        "save_setup",
        Action::SaveSetup,
        true,
        ActionCategory::System,
    ),
    d("user_menu", Action::UserMenu, true, ActionCategory::System),
    d(
        "file_associations",
        Action::FileAssociations,
        true,
        ActionCategory::Files,
    ),
    d(
        "compress_files",
        Action::CompressFiles,
        true,
        ActionCategory::Files,
    ),
    d(
        "extract_archive",
        Action::ExtractArchive,
        true,
        ActionCategory::Files,
    ),
];

/// Palette-visible catalogue rows.
pub fn palette_defs() -> impl Iterator<Item = &'static ActionDef> {
    CATALOG.iter().filter(|def| def.in_palette)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::preset::parse_action_name;

    #[test]
    fn catalog_ids_match_parse_action_name() {
        for def in CATALOG {
            assert_eq!(
                parse_action_name(def.id),
                Some(def.action),
                "id {} must parse to {:?}",
                def.id,
                def.action
            );
        }
    }

    #[test]
    fn catalog_ids_are_unique() {
        let mut ids: Vec<&str> = CATALOG.iter().map(|d| d.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), CATALOG.len());
    }

    #[test]
    fn catalog_actions_are_unique() {
        let set: std::collections::HashSet<Action> = CATALOG.iter().map(|d| d.action).collect();
        assert_eq!(set.len(), CATALOG.len());
    }

    #[test]
    fn palette_includes_copy_path_and_extract() {
        let ids: Vec<&str> = palette_defs().map(|d| d.id).collect();
        assert!(ids.contains(&"copy_path"));
        assert!(ids.contains(&"extract_archive"));
        assert_eq!(ids.len(), CATALOG.len());
    }
}
