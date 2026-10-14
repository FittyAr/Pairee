pub mod cli_line;
pub mod dialog_stack;
pub mod dir_sizes;
pub mod file_clipboard;
pub use crate::fs::glob;
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
pub use glob::{Glob, glob_matches};
pub use history::HistoryState;
pub use panel::PanelState;
pub use panel_pair::PanelPair;
pub use plugin_host::{PendingPluginReply, PluginHostState};
pub use popup::{
    ConfigDraft, ConfigurationDialogState, ConnectionPromptState, CopyMovePromptState,
    GitPanelState, PluginMenuState, PopupType,
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

/// Dirty-flag rendering: the main loop paints only when something changed.
#[derive(Debug)]
pub struct Redraw {
    /// When false, the main loop may skip `terminal.draw`.
    pub dirty: bool,
    /// Full-screen clear on next paint (resize or native TTY/admin restore).
    /// Applied inside the synchronized-update region to avoid a visible flash.
    pub needs_clear: bool,
    /// Last time transfer progress forced a redraw (rate-limit).
    pub last_transfer_draw: Option<std::time::Instant>,
}

impl Default for Redraw {
    fn default() -> Self {
        Self {
            dirty: true,
            needs_clear: false,
            last_transfer_draw: None,
        }
    }
}

/// Which modifier row of the F-key bar is shown.
#[derive(Debug, Clone, Copy)]
pub struct KeyBarMods {
    /// Modifiers held right now (as the terminal reports them).
    pub held: crossterm::event::KeyModifiers,
    /// Row picked with `cycle_fkeys_modifiers` or a click (wins over `held`).
    pub pinned: Option<crossterm::event::KeyModifiers>,
}

impl Default for KeyBarMods {
    fn default() -> Self {
        Self {
            held: crossterm::event::KeyModifiers::empty(),
            pinned: None,
        }
    }
}

/// Selection saved for RestoreSelection: the marked paths and their order.
#[derive(Debug, Clone, Default)]
pub struct SelectionSnapshot {
    pub paths: HashSet<PathBuf>,
    pub order: Vec<PathBuf>,
}

/// What a background connection attempt gives: the connection and the
/// folder to open.
pub type RemoteConnectResult = anyhow::Result<(crate::fs::remote::RemoteConn, PathBuf)>;

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
    /// Background remote connection attempt (target tab, the connection
    /// and the folder to show).
    pub remote_connect: crate::app::jobs::JobSlot<(TabId, RemoteConnectResult)>,
    /// Credential requests of background jobs and their prompts.
    pub credential_requests: crate::app::credential_requests::CredentialRequests,
    /// Background Find file search (matches so far as progress).
    pub search: crate::app::jobs::file_search::SearchSlot,
    pub plugins: PluginHostState,
    /// When the next frame must be painted (dirty-flag rendering).
    pub redraw: Redraw,

    /// Panels, viewers, editors and terminals the user switches between.
    pub screens: ScreenStack,

    // ── Folder shortcuts: number 1–9 → absolute path ─────────────────────────
    pub folder_shortcuts: HashMap<u8, PathBuf>,

    /// Selection saved for RestoreSelection.
    pub selection_snapshot: SelectionSnapshot,

    /// Panel options mirrored from the settings ([`Self::apply_settings`]).
    pub prefs: PanelPrefs,

    /// Modifier row the F-key bar shows.
    pub key_bar: KeyBarMods,
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
        crate::app::jobs::JobQueue<crate::app::actions::fs_ops::multi_rename::RenameRun>,

    /// Operation journal (undo/redo of file operations).
    pub journal: crate::fs::journal::Journal,

    /// Background change to a non-local panel source (see `vfs_op`).
    pub vfs_op: vfs_op::VfsOps,

    /// Background folder comparison (Compare folders / Synchronize).
    pub folder_scan: crate::app::sync::FolderScanState,
    /// Disk usage view: background scan and cached size tree.
    pub disk_usage: crate::app::disk_usage::DiskUsageState,
    /// Duplicate finder: background search and the marked files.
    pub dupes: crate::app::dupes::DupesState,
    /// Comparison of two files (diff or hashes), computed in the background.
    pub file_compare: crate::app::file_compare::FileCompareState,
    /// Frecency of the visited folders (folder jump, `z`).
    pub frecency: crate::config::frecency::FrecencyStore,
    /// Folder monitors that reread panels when their folder changes.
    pub auto_refresh: crate::app::auto_refresh::AutoRefresh,
    /// Monitor of the user's syntax definitions folder (reloaded on change).
    pub syntax_watch: crate::app::syntax_reload::SyntaxWatch,

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
            remote_connect: Default::default(),
            credential_requests: Default::default(),
            search: Default::default(),
            plugins: PluginHostState::default(),
            screens: ScreenStack::default(),
            folder_shortcuts: HashMap::new(),
            selection_snapshot: SelectionSnapshot::default(),
            prefs: PanelPrefs::default(),
            key_bar: KeyBarMods::default(),
            redraw: Redraw::default(),
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
            dupes: Default::default(),
            file_compare: Default::default(),
            frecency: Default::default(),
            auto_refresh: Default::default(),
            syntax_watch: Default::default(),
            scrollbar: crate::ui::scrollbar::ScrollbarUiState::default(),
            hits: Default::default(),
            mouse_released: false,
            mouse: Default::default(),
            editor_clipboard: Default::default(),
        }
    }

    pub fn mark_ui_dirty(&mut self) {
        self.redraw.dirty = true;
    }

    /// True when the frame should be painted this tick.
    /// Transfer/update progress must call [`Self::mark_ui_dirty`] (rate-limited in the main loop).
    pub fn needs_redraw(&self) -> bool {
        self.redraw.dirty || self.redraw.needs_clear
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
        let current = self.get_active_panel().current_path.clone();
        crate::app::input::cli_cd::go_to_local(self, current, target);
        self.refresh_active_panel(show_hidden);
    }

    /// Restores the last saved selection snapshot, limited to the entries
    /// the active panel lists (it may show another folder by now: file
    /// operations must never act on files that are not in view).
    pub fn restore_selection(&mut self) {
        let snapshot = &self.selection_snapshot;
        let panel = self.panels.active_mut();
        let listed: std::collections::HashSet<&PathBuf> =
            panel.entries.iter().map(|e| &e.path).collect();
        let order: Vec<PathBuf> = snapshot
            .order
            .iter()
            .filter(|p| listed.contains(p))
            .cloned()
            .collect();
        let paths = snapshot
            .paths
            .iter()
            .filter(|p| listed.contains(p))
            .cloned()
            .collect();
        panel.selected_paths = paths;
        panel.selection_order = order;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn needs_redraw_when_only_terminal_needs_clear() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.redraw.dirty = false;
        assert!(!state.needs_redraw());
        state.redraw.needs_clear = true;
        assert!(state.needs_redraw());
    }

    #[test]
    fn restored_selection_only_tags_listed_entries() {
        let entry = |path: &str| crate::fs::FileEntry {
            name: path.rsplit('/').next().unwrap().to_string(),
            path: PathBuf::from(path),
            is_dir: false,
            is_symlink: false,
            size: 0,
            modified: None,
        };
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        let panel = state.get_active_panel_mut();
        panel.entries = vec![entry("/a/x"), entry("/a/y")];
        panel.toggle_selection_with_opts(true);
        panel.cursor_index = 1;
        panel.toggle_selection_with_opts(true);
        state.snapshot_selection();
        let panel = state.get_active_panel_mut();
        panel.clear_selection();
        panel.entries = vec![entry("/b/z"), entry("/a/y")];
        state.restore_selection();
        let panel = state.get_active_panel();
        assert_eq!(panel.selection_order, [PathBuf::from("/a/y")]);
        assert_eq!(panel.get_targeted_paths(), [PathBuf::from("/a/y")]);
    }
}
