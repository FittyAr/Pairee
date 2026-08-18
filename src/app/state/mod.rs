pub mod dialog_stack;
pub mod glob;
pub mod history;
pub mod panel;
pub mod panel_pair;
pub mod plugin_host;
pub mod popup;
pub mod transfer_state;
pub mod types;
pub mod update_state;

pub mod quick_view;
pub mod refresh;
pub mod screens;

pub use crate::fs::compare::CompareStatus;
pub use dialog_stack::DialogStack;
pub use glob::{glob_matches, glob_matches_case};
pub use history::HistoryState;
pub use panel::PanelState;
pub use panel_pair::PanelPair;
pub use plugin_host::{PendingPluginReply, PluginHostState};
pub use popup::PopupType;
pub use transfer_state::{TransferTab, TransferUIState, TransferViewMode};
pub use types::{
    ActivePanel, AdminOpKind, DevProgress, FileAttrsSnapshot, GitConfirmedAction, LinkKind,
    PanelViewMode, ProcessEntry, Screen, SelectMode, SortField, TerminalUpdate, TreeNode,
};
pub use update_state::UpdateState;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct AppState {
    pub panels: PanelPair,
    pub history: HistoryState,
    pub update: UpdateState,
    pub cli_input: String,
    /// Overlay dialogs (top frame is the active popup).
    pub dialogs: DialogStack,
    pub should_quit: bool,
    /// Channel receiver for background SSH connection attempts
    pub ssh_connect_rx: Option<
        tokio::sync::oneshot::Receiver<(
            ActivePanel,
            anyhow::Result<crate::fs::ssh::SharedSshClient>,
        )>,
    >,
    /// Channel receiver for running background file search operations
    pub search_rx: Option<tokio::sync::mpsc::Receiver<(PathBuf, bool)>>,
    pub plugins: PluginHostState,
    /// Channel for communicating with the background terminal
    pub term_tx: tokio::sync::mpsc::UnboundedSender<TerminalUpdate>,
    pub term_rx: Option<tokio::sync::mpsc::UnboundedReceiver<TerminalUpdate>>,
    pub terminal_needs_clear: bool,
    /// When false, the main loop may skip `terminal.draw` (dirty-flag rendering).
    pub ui_dirty: bool,
    /// Last time transfer progress forced a redraw (rate-limit).
    pub last_transfer_draw: Option<std::time::Instant>,

    // ── Screens Management ────────────────────────────────────────────────────
    pub screens: Vec<Screen>,
    pub screen_dialogs: Vec<DialogStack>,
    pub active_screen_idx: usize,

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

    // ── Transfer Engine ───────────────────────────────────────────
    pub transfer: Option<TransferUIState>,

    // ── Scrollbar mouse hit-testing (filled each paint) ───────────
    pub scrollbar: crate::ui::scrollbar::ScrollbarUiState,
}

impl AppState {
    pub fn new(left_path: PathBuf, right_path: PathBuf) -> Self {
        let (term_tx, term_rx) = tokio::sync::mpsc::unbounded_channel();
        let is_root = crate::fs::is_elevated();
        Self {
            panels: PanelPair::new(left_path, right_path),
            history: HistoryState::default(),
            update: UpdateState::default(),
            cli_input: String::new(),
            dialogs: DialogStack::new(),
            should_quit: false,
            ssh_connect_rx: None,
            search_rx: None,
            plugins: PluginHostState::default(),
            term_tx,
            term_rx: Some(term_rx),
            screens: vec![Screen::Panels],
            screen_dialogs: vec![DialogStack::new()],
            active_screen_idx: 0,
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
            ui_dirty: true,
            last_transfer_draw: None,
            pending_custom_command: None,
            is_root,
            // Transfer Engine
            transfer: None,
            scrollbar: crate::ui::scrollbar::ScrollbarUiState::default(),
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
