use super::glob::glob_matches;
use super::types::{PanelViewMode, SortField};
use crate::fs::FileEntry;
use std::collections::HashSet;
use std::path::PathBuf;

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
    /// The order in which paths were selected (tagged) by the user
    pub selection_order: Vec<PathBuf>,
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
            selection_order: Vec::new(),
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
    /// `select_folders` controls whether directory entries can be tagged.
    pub fn toggle_selection_with_opts(&mut self, select_folders: bool) {
        if let Some(entry) = self.entries.get(self.cursor_index) {
            if entry.name != ".." && (!entry.is_dir || select_folders) {
                let path = entry.path.clone();
                if self.selected_paths.contains(&path) {
                    self.selected_paths.remove(&path);
                    self.selection_order.retain(|p| p != &path);
                } else {
                    self.selected_paths.insert(path.clone());
                    self.selection_order.push(path);
                }
            }
        }
    }

    /// Selects all entries matching a glob mask.
    pub fn select_group(&mut self, mask: &str) {
        for entry in &self.entries {
            if entry.name != ".." && glob_matches(mask, &entry.name) {
                if self.selected_paths.insert(entry.path.clone()) {
                    self.selection_order.push(entry.path.clone());
                }
            }
        }
    }

    /// Deselects all entries matching a glob mask.
    pub fn unselect_group(&mut self, mask: &str) {
        self.selected_paths
            .retain(|p| !glob_matches(mask, &p.file_name().unwrap_or_default().to_string_lossy()));
        self.selection_order
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
        // Rebuild selection_order keeping directory list order
        self.selection_order.clear();
        for entry in &self.entries {
            if self.selected_paths.contains(&entry.path) {
                self.selection_order.push(entry.path.clone());
            }
        }
    }

    /// Clears both selected_paths and selection_order
    pub fn clear_selection(&mut self) {
        self.selected_paths.clear();
        self.selection_order.clear();
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_selection_order() {
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

        // Toggle selection on a.rs (index 0)
        panel.cursor_index = 0;
        panel.toggle_selection_with_opts(false);
        assert_eq!(panel.selection_order, vec![PathBuf::from("/tmp/a.rs")]);

        // Toggle selection on b.rs (index 1)
        panel.cursor_index = 1;
        panel.toggle_selection_with_opts(false);
        assert_eq!(
            panel.selection_order,
            vec![PathBuf::from("/tmp/a.rs"), PathBuf::from("/tmp/b.rs")]
        );

        // Toggle selection on a.rs again (deselects it)
        panel.cursor_index = 0;
        panel.toggle_selection_with_opts(false);
        assert_eq!(panel.selection_order, vec![PathBuf::from("/tmp/b.rs")]);

        // Clear selection
        panel.clear_selection();
        assert!(panel.selection_order.is_empty());
        assert!(panel.selected_paths.is_empty());
    }
}
