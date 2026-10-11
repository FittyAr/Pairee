use super::PopupType;

impl PopupType {
    /// A click outside closes it (as `Esc` would): menus, pickers and
    /// read-only panels. Dialogs that hold typed input or ask for a
    /// decision stay open so a stray click cannot discard them.
    pub fn closes_on_outside_click(&self) -> bool {
        matches!(
            self,
            PopupType::Help { .. }
                | PopupType::About { .. }
                | PopupType::Error(_)
                | PopupType::Info(_)
                | PopupType::InfoPanel { .. }
                | PopupType::UserMenu { .. }
                | PopupType::Menu { .. }
                | PopupType::YaziSortPopup
                | PopupType::YaziViewPopup
                | PopupType::ContextMenu { .. }
                | PopupType::DriveSelect { .. }
                | PopupType::Hotlist { .. }
                | PopupType::FolderShortcuts { .. }
                | PopupType::PluginMenu(_)
                | PopupType::SortModesDialog { .. }
                | PopupType::ScreensMenu { .. }
                | PopupType::ArchiveCommandsMenu { .. }
                | PopupType::CommandHistoryList { .. }
                | PopupType::FileViewHistoryList { .. }
                | PopupType::FoldersHistoryList { .. }
                | PopupType::CommandPalette { .. }
                | PopupType::ViewerEncoding { .. }
                | PopupType::TreeView { .. }
                | PopupType::SelectDevPlugin { .. }
                | PopupType::UpdateAvailable { .. }
        )
    }
}
