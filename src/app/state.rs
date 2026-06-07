use crate::fs::{self, FileEntry, ProgressUpdate};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct PanelState {
    /// Absolute directory path currently listed in the panel
    pub current_path: PathBuf,
    /// List of file entries inside the directory
    pub entries: Vec<FileEntry>,
    /// Index of the currently highlighted item
    pub cursor_index: usize,
    /// Set of file/folder paths selected (tagged) by the user
    pub selected_paths: HashSet<PathBuf>,
}

impl PanelState {
    pub fn new(path: PathBuf) -> Self {
        Self {
            current_path: path,
            entries: Vec::new(),
            cursor_index: 0,
            selected_paths: HashSet::new(),
        }
    }

    /// Moves the cursor index up by one, wrapping at boundaries.
    pub fn move_cursor_up(&mut self) {
        if !self.entries.is_empty() {
            if self.cursor_index > 0 {
                self.cursor_index -= 1;
            } else {
                self.cursor_index = self.entries.len() - 1;
            }
        }
    }

    /// Moves the cursor index down by one, wrapping at boundaries.
    pub fn move_cursor_down(&mut self) {
        if !self.entries.is_empty() {
            if self.cursor_index < self.entries.len() - 1 {
                self.cursor_index += 1;
            } else {
                self.cursor_index = 0;
            }
        }
    }

    /// Moves the cursor index up by a page size.
    pub fn page_up(&mut self, page_size: usize) {
        if !self.entries.is_empty() {
            self.cursor_index = self.cursor_index.saturating_sub(page_size);
        }
    }

    /// Moves the cursor index down by a page size.
    pub fn page_down(&mut self, page_size: usize) {
        if !self.entries.is_empty() {
            self.cursor_index =
                std::cmp::min(self.cursor_index + page_size, self.entries.len() - 1);
        }
    }

    /// Moves the cursor to the first element.
    pub fn go_to_top(&mut self) {
        self.cursor_index = 0;
    }

    /// Moves the cursor to the last element.
    pub fn go_to_bottom(&mut self) {
        if !self.entries.is_empty() {
            self.cursor_index = self.entries.len() - 1;
        }
    }

    /// Selects / tags the highlighted entry.
    pub fn toggle_selection(&mut self) {
        if let Some(entry) = self.entries.get(self.cursor_index) {
            // Ignore ".." folder for multi-selection
            if entry.name != ".." {
                let path = entry.path.clone();
                if self.selected_paths.contains(&path) {
                    self.selected_paths.remove(&path);
                } else {
                    self.selected_paths.insert(path);
                }
            }
        }
    }

    /// Returns a list of paths representing the targeted items:
    /// - If any items are explicitly tagged/selected, returns those paths.
    /// - Otherwise, returns the currently highlighted item path (excluding "..").
    pub fn get_targeted_paths(&self) -> Vec<PathBuf> {
        if !self.selected_paths.is_empty() {
            self.selected_paths.iter().cloned().collect()
        } else if let Some(entry) = self.entries.get(self.cursor_index) {
            if entry.name != ".." {
                vec![entry.path.clone()]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }
}

#[derive(Debug, Clone)]
pub enum PopupType {
    Help,
    MkDirPrompt {
        input: String,
    },
    ConfirmDelete {
        paths: Vec<PathBuf>,
    },
    CopyProgress {
        current_file: String,
        files_copied: usize,
        total_files: usize,
        bytes_copied: u64,
        total_bytes: u64,
    },
    Error(String),
    UserMenu,
    InternalEditor {
        path: PathBuf,
        lines: Vec<String>,
        cursor_x: usize,
        cursor_y: usize,
        scroll_y: usize,
        is_dirty: bool,
    },
    Menu {
        active_menu_idx: usize,
        active_item_idx: usize,
    },
    DriveSelect {
        panel: ActivePanel,
        drives: Vec<String>,
        cursor_idx: usize,
    },
    Hotlist {
        bookmarks: Vec<(String, std::path::PathBuf)>,
        cursor_idx: usize,
    },
}

pub struct AppState {
    pub left_panel: PanelState,
    pub right_panel: PanelState,
    pub active_panel: ActivePanel,
    pub cli_input: String,
    pub active_popup: Option<PopupType>,
    pub should_quit: bool,
    /// Channel receiver for running copy/move operations
    pub progress_rx: Option<tokio::sync::mpsc::Receiver<ProgressUpdate>>,
}

impl AppState {
    pub fn new(left_path: PathBuf, right_path: PathBuf) -> Self {
        Self {
            left_panel: PanelState::new(left_path),
            right_panel: PanelState::new(right_path),
            active_panel: ActivePanel::Left,
            cli_input: String::new(),
            active_popup: None,
            should_quit: false,
            progress_rx: None,
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

    /// Returns a mutable reference to the passive panel state.
    #[allow(dead_code)]
    pub fn get_passive_panel_mut(&mut self) -> &mut PanelState {
        match self.active_panel {
            ActivePanel::Left => &mut self.right_panel,
            ActivePanel::Right => &mut self.left_panel,
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

    /// Refreshes directories inside left and right panels.
    pub fn refresh_both_panels(&mut self, show_hidden: bool) {
        let left_path = self.left_panel.current_path.clone();
        if let Ok(entries) = fs::read_directory(&left_path, show_hidden) {
            self.left_panel.entries = entries;
            // Check if cursor index is still within bounds
            if self.left_panel.cursor_index >= self.left_panel.entries.len() {
                self.left_panel.cursor_index = self.left_panel.entries.len().saturating_sub(1);
            }
        }

        let right_path = self.right_panel.current_path.clone();
        if let Ok(entries) = fs::read_directory(&right_path, show_hidden) {
            self.right_panel.entries = entries;
            // Check if cursor index is still within bounds
            if self.right_panel.cursor_index >= self.right_panel.entries.len() {
                self.right_panel.cursor_index = self.right_panel.entries.len().saturating_sub(1);
            }
        }
    }
}
