use super::PopupType;

impl PopupType {
    /// A click outside closes it (as `Esc` would): menus, pickers and
    /// read-only panels. Dialogs that hold typed input or ask for a
    /// decision stay open so a stray click cannot discard them.
    pub fn closes_on_outside_click(&self) -> bool {
        if let Some(dialog) = self.dialog() {
            return dialog.closes_on_outside_click();
        }
        matches!(
            self,
            PopupType::UserMenu { .. }
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

    /// Its content flows into whatever size it gets, so the user may resize
    /// it (by its bottom-right corner).
    pub fn resizable(&self) -> bool {
        if let Some(dialog) = self.dialog() {
            return dialog.resizable();
        }
        matches!(
            self,
            PopupType::UpdateAvailable { .. }
                | PopupType::GitPanel(_)
                | PopupType::SearchResults { .. }
                | PopupType::CommandHistoryList { .. }
                | PopupType::FileViewHistoryList { .. }
                | PopupType::FoldersHistoryList { .. }
                | PopupType::CompareFoldersResult { .. }
                | PopupType::TaskListDialog { .. }
                | PopupType::TreeView { .. }
                | PopupType::TransferPanel
                | PopupType::CommandPalette { .. }
                | PopupType::Shortcuts(_)
                | PopupType::DiskUsage
                | PopupType::SyncDirs(_)
        )
    }
}
