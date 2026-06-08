use crate::fs::{self, FileEntry, ProgressUpdate};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// Panel focus
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Left,
    Right,
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel view modes (mirrors NC/Far Manager Left/Right menu options)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelViewMode {
    /// Filename-only, multi-column (Ctrl+1)
    Brief,
    /// Name + basic attributes (Ctrl+2)
    Medium,
    /// Name + size + date (Ctrl+3)
    Full,
    /// Wide single column (Ctrl+4)
    Wide,
    /// Name + permissions + owner + real size (Ctrl+5)
    Detailed,
    /// Name + descript.ion entry (Ctrl+6)
    Descriptions,
    /// Name + owner/group (Ctrl+7)
    FileOwners,
    /// Name + hardlink count (Ctrl+8)
    FileLinks,
    /// User-configurable columns (Ctrl+9)
    AltFull,
}

impl Default for PanelViewMode {
    fn default() -> Self {
        PanelViewMode::Full
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sort field
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    Name,
    Extension,
    Size,
    Date,
    Unsorted,
}

impl Default for SortField {
    fn default() -> Self {
        SortField::Name
    }
}

// Re-export compare types from fs module so app/state consumers have one import path.
pub use crate::fs::compare::{CompareEntry, CompareStatus};

// File attribute snapshot (cross-platform subset)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FileAttrsSnapshot {
    pub path: PathBuf,
    pub readonly: bool,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
    pub created: Option<std::time::SystemTime>,
    pub owner: String,
    pub nlinks: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// OS Process entry (for TaskList popup)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub memory_kb: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tree view node
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub depth: usize,
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Popup types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SelectMode {
    Add,
    Remove,
}

#[derive(Debug, Clone)]
pub enum LinkKind {
    Symbolic,
    Hard,
}

#[derive(Debug, Clone)]
pub enum PopupType {
    // ── Basic ────────────────────────────────────────────────────────────────
    Help,
    Error(String),
    /// Neutral informational dialog (not an error).
    Info(String),

    // ── Prompts ──────────────────────────────────────────────────────────────
    MkDirPrompt {
        input: String,
    },
    /// Rename/Move prompt — user edits the destination path before committing.
    RenMovPrompt {
        input: String,
        src_paths: Vec<PathBuf>,
        dest_dir: PathBuf,
    },
    /// Prompt for choosing compression archive name.
    CompressPrompt {
        input: String,
        targets: Vec<PathBuf>,
        dest_dir: PathBuf,
    },
    /// Apply command template to selected files.
    ApplyCommandPrompt {
        input: String,
        targets: Vec<PathBuf>,
    },
    /// Add/edit description for a file.
    DescribeFilePrompt {
        path: PathBuf,
        current_desc: String,
        input: String,
    },
    /// Select/unselect files by glob mask.
    SelectGroupPrompt {
        mode: SelectMode,
        query: String,
    },
    /// Create a symlink or hardlink.
    CreateLinkPrompt {
        src: PathBuf,
        dest_input: String,
        kind: LinkKind,
    },
    /// File mask filter for the active panel.
    FilePanelFilterPrompt {
        input: String,
    },

    // ── Confirmations ─────────────────────────────────────────────────────────
    ConfirmDelete {
        paths: Vec<PathBuf>,
    },
    WipeConfirm {
        paths: Vec<PathBuf>,
    },
    SaveSetupConfirm,

    // ── Progress ──────────────────────────────────────────────────────────────
    CopyProgress {
        current_file: String,
        files_copied: usize,
        total_files: usize,
        bytes_copied: u64,
        total_bytes: u64,
    },

    // ── Menus / lists ─────────────────────────────────────────────────────────
    UserMenu,
    Menu {
        active_menu_idx: usize,
        active_item_idx: usize,
    },
    ContextMenu {
        items: Vec<String>,
        cursor_idx: usize,
    },
    DriveSelect {
        panel: ActivePanel,
        drives: Vec<String>,
        cursor_idx: usize,
    },
    Hotlist {
        bookmarks: Vec<(String, PathBuf)>,
        cursor_idx: usize,
    },

    // ── Sort modes ────────────────────────────────────────────────────────────
    SortModesDialog {
        current: SortField,
        reverse: bool,
        cursor_idx: usize,
    },

    // ── Editors / viewers ─────────────────────────────────────────────────────
    InternalEditor {
        path: PathBuf,
        lines: Vec<String>,
        cursor_x: usize,
        cursor_y: usize,
        scroll_y: usize,
        is_dirty: bool,
        last_search: Option<String>,
    },
    EditorSearchPrompt {
        path: PathBuf,
        lines: Vec<String>,
        cursor_x: usize,
        cursor_y: usize,
        scroll_y: usize,
        is_dirty: bool,
        last_search: Option<String>,
        query: String,
    },
    InternalViewer {
        viewer: crate::ui::viewer::ViewerState,
    },
    QuickViewPanel {
        path: PathBuf,
        content: Vec<String>,
        scroll: usize,
    },

    // ── File info ─────────────────────────────────────────────────────────────
    InfoPanel {
        lines: Vec<String>,
    },
    FileAttributesDialog {
        attrs: FileAttrsSnapshot,
        mode_input: String,
    },

    // ── Search ────────────────────────────────────────────────────────────────
    SearchPrompt {
        query: String,
        content_query: String,
        search_root: PathBuf,
        focus_content: bool,
    },
    SearchResults {
        query: String,
        results: Vec<PathBuf>,
        cursor_idx: usize,
    },

    // ── History ───────────────────────────────────────────────────────────────
    CommandHistoryList {
        entries: Vec<String>,
        cursor_idx: usize,
    },
    FileViewHistoryList {
        entries: Vec<PathBuf>,
        cursor_idx: usize,
    },
    FoldersHistoryList {
        entries: Vec<PathBuf>,
        cursor_idx: usize,
    },

    // ── Compare ───────────────────────────────────────────────────────────────
    CompareFoldersResult {
        diff: Vec<CompareEntry>,
        cursor_idx: usize,
    },

    // ── OS tools ─────────────────────────────────────────────────────────────
    TaskListDialog {
        tasks: Vec<ProcessEntry>,
        cursor_idx: usize,
    },

    // ── File associations ─────────────────────────────────────────────────────
    FileAssociationsDialog {
        rules: Vec<crate::config::associations::AssocRule>,
        cursor_idx: usize,
    },

    TreeView {
        nodes: Vec<TreeNode>,
        cursor_idx: usize,
        panel: ActivePanel,
    },

    // ── Archive commands ──────────────────────────────────────────────────────
    ArchiveCommandsMenu {
        archive_path: PathBuf,
        items: Vec<String>,
        cursor_idx: usize,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel state
// ─────────────────────────────────────────────────────────────────────────────

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
    /// Active view mode for this panel
    pub view_mode: PanelViewMode,
    /// Active sort field
    pub sort_field: SortField,
    /// Sort in reverse order
    pub sort_reverse: bool,
    /// Show full long names (true) or truncate to column width (false)
    pub show_long_names: bool,
    /// Permanent mask filter (None = show all)
    pub filter_mask: Option<String>,
}

impl PanelState {
    pub fn new(path: PathBuf) -> Self {
        Self {
            current_path: path,
            entries: Vec::new(),
            cursor_index: 0,
            selected_paths: HashSet::new(),
            view_mode: PanelViewMode::default(),
            sort_field: SortField::default(),
            sort_reverse: false,
            show_long_names: true,
            filter_mask: None,
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

    /// Selects all entries matching a glob mask.
    pub fn select_group(&mut self, mask: &str) {
        for entry in &self.entries {
            if entry.name != ".." && glob_matches(mask, &entry.name) {
                self.selected_paths.insert(entry.path.clone());
            }
        }
    }

    /// Deselects all entries matching a glob mask.
    pub fn unselect_group(&mut self, mask: &str) {
        self.selected_paths
            .retain(|p| !glob_matches(mask, &p.file_name().unwrap_or_default().to_string_lossy()));
    }

    /// Inverts the selection state of all non-".." entries.
    pub fn invert_selection(&mut self) {
        let all: HashSet<PathBuf> = self
            .entries
            .iter()
            .filter(|e| e.name != "..")
            .map(|e| e.path.clone())
            .collect();
        let currently_selected = std::mem::take(&mut self.selected_paths);
        self.selected_paths = all.difference(&currently_selected).cloned().collect();
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

// ─────────────────────────────────────────────────────────────────────────────
// App state
// ─────────────────────────────────────────────────────────────────────────────

pub struct AppState {
    pub left_panel: PanelState,
    pub right_panel: PanelState,
    pub active_panel: ActivePanel,
    pub cli_input: String,
    pub active_popup: Option<PopupType>,
    pub should_quit: bool,
    /// Channel receiver for running copy/move/extract/wipe operations
    pub progress_rx: Option<tokio::sync::mpsc::Receiver<ProgressUpdate>>,

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
            left_panel_visible: true,
            right_panel_visible: true,
            both_panels_hidden: false,
            quick_view_active: false,
            command_history: Vec::new(),
            file_view_history: Vec::new(),
            folders_history: Vec::new(),
            folder_shortcuts: HashMap::new(),
            last_selection_snapshot: HashSet::new(),
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
    }

    /// Restores the last saved selection snapshot.
    pub fn restore_selection(&mut self) {
        let snapshot = self.last_selection_snapshot.clone();
        self.get_active_panel_mut().selected_paths = snapshot;
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

    /// Refreshes directories inside left and right panels.
    pub fn refresh_both_panels(&mut self, show_hidden: bool) {
        let left_path = self.left_panel.current_path.clone();
        if let Ok(entries) = fs::read_directory(&left_path, show_hidden) {
            self.left_panel.entries = entries;
            if self.left_panel.cursor_index >= self.left_panel.entries.len() {
                self.left_panel.cursor_index = self.left_panel.entries.len().saturating_sub(1);
            }
        }

        let right_path = self.right_panel.current_path.clone();
        if let Ok(entries) = fs::read_directory(&right_path, show_hidden) {
            self.right_panel.entries = entries;
            if self.right_panel.cursor_index >= self.right_panel.entries.len() {
                self.right_panel.cursor_index =
                    self.right_panel.entries.len().saturating_sub(1);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility: simple glob matching (* and ? wildcards only)
// ─────────────────────────────────────────────────────────────────────────────

/// Matches `name` against a shell-style glob pattern supporting `*`, `?`, and `{a,b}` brace expansion.
pub fn glob_matches(pattern: &str, name: &str) -> bool {
    for pat in expand_braces(pattern) {
        if glob_match_inner(pat.as_bytes(), name.as_bytes()) {
            return true;
        }
    }
    false
}

fn expand_braces(pattern: &str) -> Vec<String> {
    if let Some(start) = pattern.find('{') {
        if let Some(end) = pattern[start..].find('}') {
            let end = start + end;
            let pre = &pattern[..start];
            let post = &pattern[end + 1..];
            let options = &pattern[start + 1..end];
            let mut results = Vec::new();
            for opt in options.split(',') {
                let expanded = format!("{}{}{}", pre, opt, post);
                results.extend(expand_braces(&expanded));
            }
            return results;
        }
    }
    vec![pattern.to_string()]
}

fn glob_match_inner(pat: &[u8], text: &[u8]) -> bool {
    match (pat.first(), text.first()) {
        (None, None) => true,
        (Some(&b'*'), _) => {
            // Try consuming zero or more chars from text
            glob_match_inner(&pat[1..], text)
                || (!text.is_empty() && glob_match_inner(pat, &text[1..]))
        }
        (Some(&b'?'), Some(_)) => glob_match_inner(&pat[1..], &text[1..]),
        (Some(p), Some(t)) => {
            p.to_ascii_lowercase() == t.to_ascii_lowercase()
                && glob_match_inner(&pat[1..], &text[1..])
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_matches() {
        assert!(glob_matches("*.rs", "main.rs"));
        assert!(glob_matches("*.rs", "lib.rs"));
        assert!(!glob_matches("*.rs", "main.toml"));
        assert!(glob_matches("foo?ar", "foobar"));
        assert!(glob_matches("*", "anything"));
        assert!(!glob_matches("*.rs", ""));
    }

    #[test]
    fn test_invert_selection() {
        let mut panel = PanelState::new(PathBuf::from("/tmp"));
        panel.entries = vec![
            FileEntry {
                name: "a.rs".to_string(),
                path: PathBuf::from("/tmp/a.rs"),
                is_dir: false,
                is_symlink: false,
                size: 0,
                modified: None,
            },
            FileEntry {
                name: "b.rs".to_string(),
                path: PathBuf::from("/tmp/b.rs"),
                is_dir: false,
                is_symlink: false,
                size: 0,
                modified: None,
            },
        ];
        panel.selected_paths.insert(PathBuf::from("/tmp/a.rs"));
        panel.invert_selection();
        assert!(!panel.selected_paths.contains(&PathBuf::from("/tmp/a.rs")));
        assert!(panel.selected_paths.contains(&PathBuf::from("/tmp/b.rs")));
    }
}
