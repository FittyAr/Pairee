pub mod glob;
pub mod panel;
pub mod transfer_state;
pub mod types;

pub mod history;
pub mod quick_view;
pub mod refresh;
pub mod screens;

pub use crate::fs::compare::CompareStatus;
pub use glob::{glob_matches, glob_matches_case};
pub use panel::PanelState;
pub use transfer_state::{TransferTab, TransferUIState, TransferViewMode};
pub use types::{
    ActivePanel, AdminOpKind, DevProgress, FileAttrsSnapshot, GitConfirmedAction, LinkKind,
    PanelViewMode, PopupType, ProcessEntry, Screen, SelectMode, SortField, TerminalUpdate,
    TreeNode,
};

use crate::plugin::manager::PluginRequest;
use crate::update::{UpdateInfo, UpdateStatus};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct AppState {
    pub left_panel: PanelState,
    pub right_panel: PanelState,
    pub active_panel: ActivePanel,
    pub cli_input: String,
    pub active_popup: Option<PopupType>,
    /// Non-modal notification overlay (e.g. "Plugin installed").
    ///
    /// A `Toast` is rendered on top of the active popup and the panels,
    /// but does **not** consume keyboard input: the main loop keeps routing
    /// keys to the active popup/panel handlers, so the user can keep
    /// working (install another plugin, navigate, etc.) while the toast
    /// is visible. Each toast carries an optional `deadline`; when it
    /// elapses the main loop clears the slot. New toasts replace the
    /// current one (with extended deadline if needed) rather than
    /// stacking, so there is always at most one toast on screen.
    pub toast: Option<crate::app::state::types::Toast>,
    pub should_quit: bool,
    /// Pending answer from
    /// [`PopupType::PermissionPrompt`]. The input
    /// handler fills it in; the background loop
    /// drains it and drives the elevated helper
    /// based on the choice.
    pub pending_permission_answer: Option<(
        uuid::Uuid,
        Vec<std::path::PathBuf>,
        crate::app::state::types::PermissionAnswer,
    )>,
    /// Channel receiver for background SSH connection attempts
    pub ssh_connect_rx: Option<
        tokio::sync::oneshot::Receiver<(
            ActivePanel,
            anyhow::Result<crate::fs::ssh::SharedSshClient>,
        )>,
    >,
    /// Channel receiver for running background file search operations
    pub search_rx: Option<tokio::sync::mpsc::Receiver<(PathBuf, bool)>>,
    /// Channel receiver for in-progress Developer Tools operations
    /// (init / lint / package / install / submit). Drained each frame in
    /// `process_background_updates` to update the `PluginMenu` popup's
    /// progress fields without blocking the UI thread.
    pub dev_progress_rx: Option<tokio::sync::mpsc::UnboundedReceiver<DevProgress>>,
    /// Channel for communicating with the background terminal
    pub term_tx: tokio::sync::mpsc::UnboundedSender<TerminalUpdate>,
    pub term_rx: Option<tokio::sync::mpsc::UnboundedReceiver<TerminalUpdate>>,
    /// Channel receiver for plugin requests (Cd, FsRead, Notify, ...).
    /// Owned by `AppState` directly so the dispatcher in
    /// `process_plugin_requests` can `try_recv` without locking a
    /// global. The matching sender is registered into the
    /// `PLUGIN_REQ_TX` `OnceLock` by [`AppState::new`] so the rest
    /// of the app (and the plugin runtime) can publish requests
    /// without holding a reference to the state. Tests that want
    /// to inject requests should replace this field with their own
    /// receiver and use the matching sender to drive the test
    /// (no global shared between tests).
    pub plugin_req_rx: tokio::sync::mpsc::Receiver<PluginRequest>,
    pub terminal_needs_clear: bool,

    // ── Screens Management ────────────────────────────────────────────────────
    pub screens: Vec<Screen>,
    pub screen_popups: Vec<Option<PopupType>>,
    pub active_screen_idx: usize,

    // ── Panel visibility ──────────────────────────────────────────────────────
    pub left_panel_visible: bool,
    pub right_panel_visible: bool,
    /// Ctrl+O: hide both panels to reveal the full terminal output below
    pub both_panels_hidden: bool,
    /// Whether quick-view is active (passive panel shows file preview)
    pub quick_view_active: bool,

    // ── History lists (in-memory; persisted via config::history) ─────────────
    pub command_history: Vec<String>,
    pub file_view_history: Vec<PathBuf>,
    pub folders_history: Vec<PathBuf>,

    // ── Folder shortcuts: number 1–9 → absolute path ─────────────────────────
    pub folder_shortcuts: HashMap<u8, PathBuf>,

    // ── Selection snapshot for RestoreSelection ───────────────────────────────
    pub last_selection_snapshot: HashSet<PathBuf>,
    pub last_selection_order_snapshot: Vec<PathBuf>,

    // ── System settings ───────────────────────────────────────────────────────
    pub case_sensitive_sort: bool,
    pub treat_digits_as_numbers: bool,
    pub sorting_collation: String,
    pub req_admin_reading: bool,

    // ── Panel settings (mirrors Settings for quick access) ────────────────────
    pub select_folders: bool,
    pub sort_folder_names_by_extension: bool,
    pub show_dotdot_in_root_folders: bool,
    pub disable_panel_update_object_count: u32,
    pub free_space_left: Option<u64>,
    pub free_space_right: Option<u64>,

    pub current_modifiers: crossterm::event::KeyModifiers,
    pub fkeys_modifier_override: Option<crossterm::event::KeyModifiers>,
    pub pending_custom_command: Option<String>,
    pub is_root: bool,

    // ── Auto-update ────────────────────────────────────────────────
    /// Pending oneshot receiver for the background update check.
    pub update_check_rx: Option<tokio::sync::oneshot::Receiver<Option<crate::update::UpdateInfo>>>,
    /// Available update info (set after the background check completes).
    pub update_available: Option<UpdateInfo>,
    /// Current status of an ongoing update installation.
    pub update_status: UpdateStatus,
    /// Receiver for download progress (0.0–1.0).
    pub update_progress_rx: Option<tokio::sync::mpsc::Receiver<f32>>,
    /// Pending oneshot receiver for the final installation result.
    pub update_install_rx: Option<
        tokio::sync::oneshot::Receiver<Result<crate::update::installer::InstallResult, String>>,
    >,
    // ── Transfer Engine ───────────────────────────────────────────
    pub transfer: Option<TransferUIState>,
}

impl AppState {
    pub fn new(left_path: PathBuf, right_path: PathBuf) -> Self {
        let (term_tx, term_rx) = tokio::sync::mpsc::unbounded_channel();
        // Create the plugin request channel pair. The receiver
        // is owned by the state (so `process_plugin_requests`
        // can `try_recv` without locking a global); the
        // sender is registered into the `PLUGIN_REQ_TX`
        // `OnceLock` (re-exported at `crate::plugin::manager`)
        // so the plugin runtime and other call sites can
        // publish requests without holding a reference to
        // the state.
        let (plugin_tx, plugin_rx) = tokio::sync::mpsc::channel(100);
        let _ = crate::plugin::manager::PLUGIN_REQ_TX.set(plugin_tx);
        let is_root = crate::fs::is_elevated();
        Self {
            left_panel: PanelState::new(left_path),
            right_panel: PanelState::new(right_path),
            active_panel: ActivePanel::Left,
            cli_input: String::new(),
            active_popup: None,
            toast: None,
            should_quit: false,
            pending_permission_answer: None,
            ssh_connect_rx: None,
            search_rx: None,
            dev_progress_rx: None,
            term_tx,
            term_rx: Some(term_rx),
            plugin_req_rx: plugin_rx,
            screens: vec![Screen::Panels],
            screen_popups: vec![None],
            active_screen_idx: 0,
            left_panel_visible: true,
            right_panel_visible: true,
            both_panels_hidden: false,
            quick_view_active: false,
            command_history: Vec::new(),
            file_view_history: Vec::new(),
            folders_history: Vec::new(),
            folder_shortcuts: HashMap::new(),
            last_selection_snapshot: HashSet::new(),
            last_selection_order_snapshot: Vec::new(),
            case_sensitive_sort: false,
            treat_digits_as_numbers: false,
            sorting_collation: "linguistic".to_string(),
            req_admin_reading: false,
            select_folders: false,
            sort_folder_names_by_extension: false,
            show_dotdot_in_root_folders: false,
            disable_panel_update_object_count: 0,
            free_space_left: None,
            free_space_right: None,
            current_modifiers: crossterm::event::KeyModifiers::empty(),
            fkeys_modifier_override: None,
            terminal_needs_clear: false,
            pending_custom_command: None,
            is_root,
            // Update
            update_check_rx: None,
            update_available: None,
            update_status: UpdateStatus::Idle,
            update_progress_rx: None,
            update_install_rx: None,
            // Transfer Engine
            transfer: None,
        }
    }

    /// Returns a reference to the active panel state.
    pub fn get_active_panel(&self) -> &PanelState {
        match self.active_panel {
            ActivePanel::Left => &self.left_panel,
            ActivePanel::Right => &self.right_panel,
        }
    }

    /// Returns a mutable reference to the active panel state.
    pub fn get_active_panel_mut(&mut self) -> &mut PanelState {
        match self.active_panel {
            ActivePanel::Left => &mut self.left_panel,
            ActivePanel::Right => &mut self.right_panel,
        }
    }

    /// Returns a reference to the passive panel state.
    pub fn get_passive_panel(&self) -> &PanelState {
        match self.active_panel {
            ActivePanel::Left => &self.right_panel,
            ActivePanel::Right => &self.left_panel,
        }
    }

    /// Swaps the paths (and lists) of the left and right panels.
    pub fn swap_panels(&mut self) {
        std::mem::swap(&mut self.left_panel, &mut self.right_panel);
    }

    /// Switches keyboard focus between panels.
    pub fn toggle_focus(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Left => ActivePanel::Right,
            ActivePanel::Right => ActivePanel::Left,
        };
    }

    /// Saves the current selection snapshot for later RestoreSelection.
    pub fn snapshot_selection(&mut self) {
        self.last_selection_snapshot = self.get_active_panel().selected_paths.clone();
        self.last_selection_order_snapshot = self.get_active_panel().selection_order.clone();
    }

    /// Restores the last saved selection snapshot.
    pub fn restore_selection(&mut self) {
        let snapshot = self.last_selection_snapshot.clone();
        self.get_active_panel_mut().selected_paths = snapshot;
        let order_snapshot = self.last_selection_order_snapshot.clone();
        self.get_active_panel_mut().selection_order = order_snapshot;
    }
}
