pub mod cli_line;
pub mod dialog_stack;
pub mod dir_sizes;
pub mod file_clipboard;
pub mod glob;
pub mod history;
pub mod nav_history;
pub mod panel;
pub mod panel_pair;
pub mod plugin_host;
pub mod popup;
pub mod prefs;
pub mod transfer_state;
pub mod types;
pub mod update_state;

pub mod quick_view;
pub mod refresh;
pub mod screens;
mod tab_ops;
pub mod tabs;
pub mod terminal;
mod vfs_op;
pub use vfs_op::VfsFollowUp;
pub mod viewer;
pub mod visual;

pub use crate::fs::compare::CompareStatus;
pub use cli_line::CliLine;
pub use dialog_stack::DialogStack;
pub use glob::{Glob, glob_matches, glob_matches_case};
pub use history::HistoryState;
pub use panel::PanelState;
pub use panel_pair::PanelPair;
pub use plugin_host::{PendingPluginReply, PluginHostState};
pub use popup::{
    ConfigDraft, ConfigurationDialogState, CopyMovePromptState, GitPanelState, PluginMenuState,
    PopupType, SshConnectPromptState,
};
pub use prefs::PanelPrefs;
pub use screens::{ScreenId, ScreenStack};
pub use tabs::TabId;
pub use terminal::TerminalState;
pub use transfer_state::{TransferTab, TransferUIState, TransferViewMode};
pub use types::{
    ActivePanel, AdminOpKind, DevProgress, FileAttrsSnapshot, GitConfirmedAction, LinkKind,
    PanelViewMode, ProcessEntry, Screen, SelectMode, SortField, TreeNode,
};
pub use update_state::UpdateState;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Selection saved for RestoreSelection: the marked paths and their order.
#[derive(Debug, Clone, Default)]
pub struct SelectionSnapshot {
    pub paths: HashSet<PathBuf>,
    pub order: Vec<PathBuf>,
}

pub struct AppState {
    pub panels: PanelPair,
    pub history: HistoryState,
    pub update: UpdateState,
    /// The command line under the panels.
    pub cli: CliLine,
    /// Letters typed for type-ahead find.
    pub type_ahead: crate::app::input::type_ahead::TypeAheadBuffer,
    /// Items yanked / cut for a later paste.
    pub file_clipboard: Option<file_clipboard::FileClipboard>,
    /// Last confirmed in-panel search (`find_next` / `find_prev`).
    pub last_panel_search: Option<crate::app::actions::panel_search::PanelQuery>,
    /// Overlay dialogs (top frame is the active popup).
    pub dialogs: DialogStack,
    pub should_quit: bool,
    /// Background SSH connection attempt (target tab, result).
    pub ssh_connect:
        crate::app::jobs::JobSlot<(TabId, anyhow::Result<crate::fs::ssh::SharedSshClient>)>,
    /// Background Find file search (matches so far as progress).
    pub search: crate::app::jobs::file_search::SearchSlot,
    pub plugins: PluginHostState,
    /// Full-screen clear on next paint (resize or native TTY/admin restore).
    /// Applied inside the synchronized-update region to avoid a visible flash.
    pub terminal_needs_clear: bool,
    /// When false, the main loop may skip `terminal.draw` (dirty-flag rendering).
    pub ui_dirty: bool,
    /// Last time transfer progress forced a redraw (rate-limit).
    pub last_transfer_draw: Option<std::time::Instant>,

    /// Panels, viewers, editors and terminals the user switches between.
    pub screens: ScreenStack,

    // ── Folder shortcuts: number 1–9 → absolute path ─────────────────────────
    pub folder_shortcuts: HashMap<u8, PathBuf>,

    /// Selection saved for RestoreSelection.
    pub selection_snapshot: SelectionSnapshot,

    /// Panel options mirrored from the settings ([`Self::apply_settings`]).
    pub prefs: PanelPrefs,

    pub current_modifiers: crossterm::event::KeyModifiers,
    pub fkeys_modifier_override: Option<crossterm::event::KeyModifiers>,
    pub pending_custom_command: Option<String>,
    pub is_root: bool,

    // ── Transfer Engine ───────────────────────────────────────────
    pub transfer: Option<TransferUIState>,

    /// Background open and search of the internal viewer.
    pub viewer: viewer::ViewerJobs,

    /// Quick-view background loading, debounce and preview cache.
    pub quick_view: quick_view::QuickViewState,

    /// Background Git network operation (fetch/pull/push/clone).
    pub git_op: crate::app::git_ops::GitOpState,

    /// Background loading of the Git panel (status, log, refs).
    pub git_panel: crate::app::git_panel_load::GitPanelLoader,

    /// Background run of the multi-rename tool.
    pub multi_rename:
        crate::app::jobs::JobSlot<crate::app::actions::fs_ops::multi_rename::RenameRun>,

    /// Operation journal (undo/redo of file operations).
    pub journal: crate::fs::journal::Journal,

    /// Background change to a non-local panel source (see `vfs_op`).
    pub vfs_op: crate::app::jobs::JobSlot<Result<vfs_op::VfsFollowUp, String>>,

    /// Background folder comparison (Compare folders / Synchronize).
    pub folder_scan: crate::app::sync::FolderScanState,
    /// Disk usage view: background scan and cached size tree.
    pub disk_usage: crate::app::disk_usage::DiskUsageState,
    /// Folder monitors that reread panels when their folder changes.
    pub auto_refresh: crate::app::auto_refresh::AutoRefresh,

    // ── Mouse (hits are recorded by each paint) ──────────────────
    pub scrollbar: crate::ui::scrollbar::ScrollbarUiState,
    /// What the last frame painted where, for the mouse router.
    pub hits: crate::ui::hit::HitMap,
    /// Mouse capture handed back to the terminal (`toggle_mouse_capture`)
    /// so it can select text; the main loop releases it while set.
    pub mouse_released: bool,
    /// Click counting and other state kept between mouse events.
    pub mouse: crate::app::mouse::MouseState,

    /// Clipboard shared by every editor screen (system + internal fallback).
    pub editor_clipboard: crate::app::editor::EditorClipboard,
}

impl AppState {
    pub fn new(left_path: PathBuf, right_path: PathBuf) -> Self {
        let is_root = crate::fs::is_elevated();
        Self {
            panels: PanelPair::new(left_path, right_path),
            history: HistoryState::default(),
            update: UpdateState::default(),
            cli: CliLine::default(),
            type_ahead: Default::default(),
            file_clipboard: None,
            last_panel_search: None,
            dialogs: DialogStack::new(),
            should_quit: false,
            ssh_connect: Default::default(),
            search: Default::default(),
            plugins: PluginHostState::default(),
            screens: ScreenStack::default(),
            folder_shortcuts: HashMap::new(),
            selection_snapshot: SelectionSnapshot::default(),
            prefs: PanelPrefs::default(),
            current_modifiers: crossterm::event::KeyModifiers::empty(),
            fkeys_modifier_override: None,
            terminal_needs_clear: false,
            ui_dirty: true,
            last_transfer_draw: None,
            pending_custom_command: None,
            is_root,
            // Transfer Engine
            transfer: None,
            viewer: Default::default(),
            quick_view: Default::default(),
            git_op: Default::default(),
            git_panel: Default::default(),
            multi_rename: Default::default(),
            vfs_op: Default::default(),
            journal: Default::default(),
            folder_scan: Default::default(),
            disk_usage: Default::default(),
            auto_refresh: Default::default(),
            scrollbar: crate::ui::scrollbar::ScrollbarUiState::default(),
            hits: Default::default(),
            mouse_released: false,
            mouse: Default::default(),
            editor_clipboard: Default::default(),
        }
    }

    pub fn mark_ui_dirty(&mut self) {
        self.ui_dirty = true;
    }

    /// True when the frame should be painted this tick.
    /// Transfer/update progress must call [`Self::mark_ui_dirty`] (rate-limited in the main loop).
    pub fn needs_redraw(&self) -> bool {
        self.ui_dirty || self.terminal_needs_clear
    }

    /// Returns a reference to the active panel state.
    pub fn get_active_panel(&self) -> &PanelState {
        self.panels.active()
    }

    /// Returns a mutable reference to the active panel state.
    pub fn get_active_panel_mut(&mut self) -> &mut PanelState {
        self.panels.active_mut()
    }

    /// Returns a reference to the passive panel state.
    pub fn get_passive_panel(&self) -> &PanelState {
        self.panels.passive()
    }

    /// Swaps the paths (and lists) of the left and right panels.
    pub fn swap_panels(&mut self) {
        self.panels.swap();
    }

    /// Switches keyboard focus between panels.
    pub fn toggle_focus(&mut self) {
        self.panels.toggle_focus();
    }

    /// Saves the current selection snapshot for later RestoreSelection.
    pub fn snapshot_selection(&mut self) {
        let panel = self.get_active_panel();
        self.selection_snapshot = SelectionSnapshot {
            paths: panel.selected_paths.clone(),
            order: panel.selection_order.clone(),
        };
    }

    /// Points the active panel at a local folder (hotlist / folder shortcut jump).
    pub fn jump_active_panel_to(&mut self, target: PathBuf, show_hidden: bool) {
        let panel = self.get_active_panel_mut();
        panel.source = crate::fs::vfs::PanelSource::Local;
        panel.open_path(target.clone());
        self.push_folders_history(target);
        self.refresh_active_panel(show_hidden);
    }

    /// Restores the last saved selection snapshot.
    pub fn restore_selection(&mut self) {
        let snapshot = self.selection_snapshot.clone();
        let panel = self.get_active_panel_mut();
        panel.selected_paths = snapshot.paths;
        panel.selection_order = snapshot.order;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn needs_redraw_when_only_terminal_needs_clear() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.ui_dirty = false;
        assert!(!state.needs_redraw());
        state.terminal_needs_clear = true;
        assert!(state.needs_redraw());
    }
}
