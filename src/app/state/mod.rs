pub mod glob;
pub mod panel;
pub mod types;

pub mod history;
pub mod quick_view;
pub mod refresh;
pub mod screens;

pub use crate::fs::compare::CompareStatus;
pub use glob::{glob_matches, glob_matches_case};
pub use panel::PanelState;
pub use types::{
    ActivePanel, AdminOpKind, BackgroundOpContext, FileAttrsSnapshot, LinkKind, PanelViewMode,
    PopupType, ProcessEntry, Screen, SelectMode, SortField, TerminalUpdate, TreeNode,
};

use crate::fs::ProgressUpdate;
use crate::update::{UpdateInfo, UpdateStatus};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct AppState {
    pub left_panel: PanelState,
    pub right_panel: PanelState,
    pub active_panel: ActivePanel,
    pub cli_input: String,
    pub active_popup: Option<PopupType>,
    pub should_quit: bool,
    /// Channel receiver for running copy/move/extract/wipe operations
    pub progress_rx: Option<tokio::sync::mpsc::Receiver<ProgressUpdate>>,
    /// Channel receiver for background SSH connection attempts
    pub ssh_connect_rx: Option<
        tokio::sync::oneshot::Receiver<(
            ActivePanel,
            anyhow::Result<crate::fs::ssh::SharedSshClient>,
        )>,
    >,
    /// Channel receiver for running background file search operations
    pub search_rx: Option<tokio::sync::mpsc::Receiver<(PathBuf, bool)>>,
    /// Channel for communicating with the background terminal
    pub term_tx: tokio::sync::mpsc::UnboundedSender<TerminalUpdate>,
    pub term_rx: Option<tokio::sync::mpsc::UnboundedReceiver<TerminalUpdate>>,
    pub active_bg_op: Option<BackgroundOpContext>,
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
    pub update_check_rx:
        Option<tokio::sync::oneshot::Receiver<Option<crate::update::UpdateInfo>>>,
    /// Available update info (set after the background check completes).
    pub update_available: Option<UpdateInfo>,
    /// Current status of an ongoing update installation.
    pub update_status: UpdateStatus,
    /// Receiver for download progress (0.0–1.0).
    pub update_progress_rx: Option<tokio::sync::mpsc::Receiver<f32>>,
    /// Pending oneshot receiver for the final installation result.
    pub update_install_rx:
        Option<tokio::sync::oneshot::Receiver<Result<crate::update::installer::InstallResult, String>>>,
}

impl AppState {
    pub fn new(left_path: PathBuf, right_path: PathBuf) -> Self {
        let (term_tx, term_rx) = tokio::sync::mpsc::unbounded_channel();
        let is_root = crate::fs::is_elevated();
        Self {
            left_panel: PanelState::new(left_path),
            right_panel: PanelState::new(right_path),
            active_panel: ActivePanel::Left,
            cli_input: String::new(),
            active_popup: None,
            should_quit: false,
            progress_rx: None,
            ssh_connect_rx: None,
            search_rx: None,
            term_tx,
            term_rx: Some(term_rx),
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
            active_bg_op: None,
            terminal_needs_clear: false,
            pending_custom_command: None,
            is_root,
            // Update
            update_check_rx: None,
            update_available: None,
            update_status: UpdateStatus::Idle,
            update_progress_rx: None,
            update_install_rx: None,
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
