pub mod glob;
pub mod panel;
pub mod types;

pub use crate::fs::compare::CompareStatus;
pub use glob::{glob_matches, glob_matches_case};
pub use panel::PanelState;
pub use types::{
    ActivePanel, AdminOpKind, BackgroundOpContext, FileAttrsSnapshot, LinkKind, PanelViewMode,
    PopupType, ProcessEntry, Screen, SelectMode, SortField, TerminalUpdate, TreeNode,
};

use crate::fs::{self, ProgressUpdate};
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

    /// Pushes a path to the file view history.
    pub fn push_file_view_history(&mut self, path: PathBuf) {
        let mut store = crate::config::history::HistoryStore {
            commands: std::mem::take(&mut self.command_history),
            viewed_files: std::mem::take(&mut self.file_view_history),
            visited_folders: std::mem::take(&mut self.folders_history),
        };
        store.push_viewed_file(path);
        self.command_history = store.commands;
        self.file_view_history = store.viewed_files;
        self.folders_history = store.visited_folders;
    }

    /// Pushes a folder to the folders history.
    pub fn push_folders_history(&mut self, path: PathBuf) {
        let mut store = crate::config::history::HistoryStore {
            commands: std::mem::take(&mut self.command_history),
            viewed_files: std::mem::take(&mut self.file_view_history),
            visited_folders: std::mem::take(&mut self.folders_history),
        };
        store.push_visited_folder(path);
        self.command_history = store.commands;
        self.file_view_history = store.viewed_files;
        self.folders_history = store.visited_folders;
    }

    /// Pushes a CLI command to the command history.
    pub fn push_command_history(&mut self, cmd: String) {
        let mut store = crate::config::history::HistoryStore {
            commands: std::mem::take(&mut self.command_history),
            viewed_files: std::mem::take(&mut self.file_view_history),
            visited_folders: std::mem::take(&mut self.folders_history),
        };
        store.push_command(cmd);
        self.command_history = store.commands;
        self.file_view_history = store.viewed_files;
        self.folders_history = store.visited_folders;
    }

    /// Refreshes directories inside left and right panels, using full panel settings.
    pub fn refresh_both_panels(&mut self, show_hidden: bool) {
        let left_path = self.left_panel.current_path.clone();
        let left_count = self.left_panel.entries.len();
        let skip_left = self.disable_panel_update_object_count > 0
            && left_count as u32 > self.disable_panel_update_object_count;
        if !skip_left {
            let res = if let Some(client) = &self.left_panel.ssh_conn {
                client.read_directory(
                    &left_path,
                    show_hidden,
                    self.case_sensitive_sort,
                    self.treat_digits_as_numbers,
                    self.left_panel.sort_field,
                    self.left_panel.sort_reverse,
                    self.show_dotdot_in_root_folders,
                )
            } else {
                fs::read_directory_ext(
                    &left_path,
                    show_hidden,
                    self.case_sensitive_sort,
                    self.treat_digits_as_numbers,
                    &self.sorting_collation,
                    self.req_admin_reading,
                    self.left_panel.sort_field,
                    self.left_panel.sort_reverse,
                    self.sort_folder_names_by_extension,
                    self.show_dotdot_in_root_folders,
                )
            };

            if let Ok(entries) = res {
                self.left_panel.entries = entries;
                if self.left_panel.cursor_index >= self.left_panel.entries.len() {
                    self.left_panel.cursor_index = self.left_panel.entries.len().saturating_sub(1);
                }
            }
            self.free_space_left = if self.left_panel.ssh_conn.is_none() {
                crate::app::sys_helpers::get_free_space(&left_path)
            } else {
                None
            };
        }

        let right_path = self.right_panel.current_path.clone();
        let right_count = self.right_panel.entries.len();
        let skip_right = self.disable_panel_update_object_count > 0
            && right_count as u32 > self.disable_panel_update_object_count;
        if !skip_right {
            let res = if let Some(client) = &self.right_panel.ssh_conn {
                client.read_directory(
                    &right_path,
                    show_hidden,
                    self.case_sensitive_sort,
                    self.treat_digits_as_numbers,
                    self.right_panel.sort_field,
                    self.right_panel.sort_reverse,
                    self.show_dotdot_in_root_folders,
                )
            } else {
                fs::read_directory_ext(
                    &right_path,
                    show_hidden,
                    self.case_sensitive_sort,
                    self.treat_digits_as_numbers,
                    &self.sorting_collation,
                    self.req_admin_reading,
                    self.right_panel.sort_field,
                    self.right_panel.sort_reverse,
                    self.sort_folder_names_by_extension,
                    self.show_dotdot_in_root_folders,
                )
            };

            if let Ok(entries) = res {
                self.right_panel.entries = entries;
                if self.right_panel.cursor_index >= self.right_panel.entries.len() {
                    self.right_panel.cursor_index =
                        self.right_panel.entries.len().saturating_sub(1);
                }
            }
            self.free_space_right = if self.right_panel.ssh_conn.is_none() {
                crate::app::sys_helpers::get_free_space(&right_path)
            } else {
                None
            };
        }
    }

    /// Adds a new screen to the stack and makes it active.
    pub fn push_screen(&mut self, screen: Screen) {
        if self.active_screen_idx < self.screen_popups.len() {
            self.screen_popups[self.active_screen_idx] = self.active_popup.take();
        }
        self.screens.push(screen);
        self.screen_popups.push(None);
        self.active_screen_idx = self.screens.len() - 1;
        self.active_popup = None;
    }

    /// Switches to the next screen (Ctrl-Tab).
    pub fn next_screen(&mut self) {
        if self.screens.len() > 1 {
            self.screen_popups[self.active_screen_idx] = self.active_popup.take();
            self.active_screen_idx = (self.active_screen_idx + 1) % self.screens.len();
            self.active_popup = self.screen_popups[self.active_screen_idx].take();
        }
    }

    /// Switches to the previous screen (Ctrl-Shift-Tab).
    pub fn prev_screen(&mut self) {
        if self.screens.len() > 1 {
            self.screen_popups[self.active_screen_idx] = self.active_popup.take();
            self.active_screen_idx = if self.active_screen_idx == 0 {
                self.screens.len() - 1
            } else {
                self.active_screen_idx - 1
            };
            self.active_popup = self.screen_popups[self.active_screen_idx].take();
        }
    }

    /// Closes the currently active screen, reverting to the previous one.
    pub fn close_current_screen(&mut self) {
        if self.active_screen_idx > 0 && self.active_screen_idx < self.screens.len() {
            self.screens.remove(self.active_screen_idx);
            self.screen_popups.remove(self.active_screen_idx);
            self.active_screen_idx -= 1;
            self.active_popup = self.screen_popups[self.active_screen_idx].take();
        }
    }

    /// Dynamically updates the quick view panel preview content.
    pub fn update_quick_view(&mut self) {
        if self.quick_view_active {
            let active = self.get_active_panel();
            let target_path = if !active.selection_order.is_empty() {
                Some(active.selection_order[0].clone())
            } else if let Some(entry) = active.entries.get(active.cursor_index) {
                Some(entry.path.clone())
            } else {
                None
            };

            if let Some(path) = target_path {
                let needs_load = match &self.active_popup {
                    Some(PopupType::QuickViewPanel {
                        path: current_path, ..
                    }) => current_path != &path,
                    _ => true,
                };

                if needs_load {
                    let is_image_ext = path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| {
                            let ext_lower = ext.to_lowercase();
                            matches!(
                                ext_lower.as_str(),
                                "png"
                                    | "jpg"
                                    | "jpeg"
                                    | "bmp"
                                    | "gif"
                                    | "webp"
                                    | "tif"
                                    | "tiff"
                                    | "ico"
                                    | "tga"
                            )
                        })
                        .unwrap_or(false);

                    let mut image_data = None;
                    if is_image_ext {
                        if let Ok(img) = image::open(&path) {
                            image_data = Some(img);
                        }
                    }

                    let is_pdf = path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.to_lowercase() == "pdf")
                        .unwrap_or(false);

                    let content = if is_pdf {
                        match std::fs::read(&path) {
                            Ok(bytes) => {
                                if let Some(pdf_text) = extract_pdf_text(&bytes) {
                                    pdf_text.lines().map(|s| s.to_string()).collect()
                                } else {
                                    vec!["[PDF file — no extractable text found]".to_string()]
                                }
                            }
                            Err(e) => vec![format!("[Error reading PDF: {}]", e)],
                        }
                    } else if image_data.is_some() {
                        Vec::new()
                    } else {
                        crate::ui::quickview::load_quick_view_content(&path)
                    };

                    self.active_popup = Some(PopupType::QuickViewPanel {
                        path,
                        content,
                        scroll: 0,
                        image_data,
                    });
                }
            } else {
                self.active_popup = None;
            }
        }
    }
}

fn extract_pdf_text(data: &[u8]) -> Option<String> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let mut text_content = String::new();
    let mut pos = 0;
    while let Some(stream_start) = find_subsequence(&data[pos..], b"stream") {
        let actual_start = pos + stream_start + 6;
        let mut data_start = actual_start;
        while data_start < data.len() && (data[data_start] == b'\r' || data[data_start] == b'\n') {
            data_start += 1;
        }

        if let Some(stream_end) = find_subsequence(&data[data_start..], b"endstream") {
            let actual_end = data_start + stream_end;
            let compressed_data = &data[data_start..actual_end];

            let mut decoder = ZlibDecoder::new(compressed_data);
            let mut decompressed = Vec::new();
            if decoder.read_to_end(&mut decompressed).is_ok() {
                let mut i = 0;
                let mut in_string = false;
                let mut current_str = Vec::new();
                let mut escaped = false;
                while i < decompressed.len() {
                    let c = decompressed[i];
                    if in_string {
                        if escaped {
                            current_str.push(c);
                            escaped = false;
                        } else if c == b'\\' {
                            escaped = true;
                        } else if c == b')' {
                            in_string = false;
                            let s = String::from_utf8_lossy(&current_str);
                            text_content.push_str(&s);
                            current_str.clear();
                        } else {
                            current_str.push(c);
                        }
                    } else if c == b'(' {
                        in_string = true;
                    } else if c == b'\n' || c == b'\r' {
                        text_content.push('\n');
                    }
                    i += 1;
                }
                text_content.push('\n');
            }
            pos = actual_end + 9;
        } else {
            break;
        }
    }

    if text_content.trim().is_empty() {
        None
    } else {
        let cleaned: Vec<String> = text_content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        Some(cleaned.join("\n"))
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
