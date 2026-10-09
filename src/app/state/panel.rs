use super::dir_sizes::DirSizes;
use super::glob::glob_matches;
use super::refresh::listing::PanelListing;
use super::types::{PanelViewMode, SortField};
use crate::app::jobs::JobSlot;
use crate::fs::FileEntry;
use crate::fs::attrs::FileAttrs;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug)]
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
    /// Quick name fragment filter
    pub quick_filter_mask: Option<String>,
    /// Last path refreshed in the panel
    pub last_path: PathBuf,
    /// Where the entries come from (local disk, SFTP server, archive).
    pub source: crate::fs::vfs::PanelSource,
    /// Active Git branch name if current_path is inside a Git repository
    pub git_branch: Option<String>,
    /// Map of entry filename -> Git status label (e.g. "M", "A", "?", "D")
    pub git_statuses: std::collections::HashMap<String, String>,
    /// Background listing job (directory read + git status + free space).
    pub listing: JobSlot<PanelListing>,
    /// The running listing is an automatic refresh: no "Loading…" title.
    pub quiet_listing: bool,
    /// Directory whose contents `entries` currently show.
    pub listed_path: Option<PathBuf>,
    /// Entry name to put the cursor on once the pending listing arrives.
    pub pending_focus: Option<String>,
    /// Per-entry attributes for detailed views, read with the listing.
    pub attrs: HashMap<PathBuf, FileAttrs>,
    /// Free space of the listed volume (local panels only).
    pub free_space: Option<u64>,
    /// Folder sizes computed on request, kept until another directory is listed.
    pub dir_sizes: DirSizes,
    /// Back / forward history of the folders this panel showed.
    pub nav: super::nav_history::NavHistory,
    /// Active visual selection (`visual_mode`).
    pub visual: Option<super::visual::VisualSelection>,
}

impl PanelState {
    pub fn new(path: PathBuf) -> Self {
        Self {
            current_path: path.clone(),
            entries: Vec::new(),
            cursor_index: 0,
            selected_paths: HashSet::new(),
            selection_order: Vec::new(),
            view_mode: PanelViewMode::default(),
            sort_field: SortField::default(),
            sort_reverse: false,
            show_long_names: true,
            filter_mask: None,
            quick_filter_mask: None,
            last_path: path,
            source: Default::default(),
            git_branch: None,
            git_statuses: std::collections::HashMap::new(),
            listing: JobSlot::new(),
            quiet_listing: false,
            listed_path: None,
            pending_focus: None,
            attrs: HashMap::new(),
            free_space: None,
            dir_sizes: DirSizes::default(),
            nav: Default::default(),
            visual: None,
        }
    }

    /// Attributes read with the last listing (`None` for remote panels or
    /// when the view mode did not request them).
    pub fn entry_attrs(&self, entry: &FileEntry) -> Option<&FileAttrs> {
        self.attrs.get(&entry.path)
    }

    /// True while a reread the user should see is running (automatic
    /// refreshes stay silent so the title does not flicker).
    pub fn is_loading(&self) -> bool {
        self.listing.is_running() && !self.quiet_listing
    }

    /// Stops the listing and folder-size jobs (the tab is going away).
    pub fn cancel_jobs(&mut self) {
        self.listing.cancel();
        self.dir_sizes.cancel();
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
        if let Some(entry) = self.entries.get(self.cursor_index)
            && entry.name != ".."
            && (!entry.is_dir || select_folders)
        {
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

    /// Selects all entries matching a glob mask.
    pub fn select_group(&mut self, mask: &str) {
        for entry in &self.entries {
            if entry.name != ".."
                && glob_matches(mask, &entry.name)
                && self.selected_paths.insert(entry.path.clone())
            {
                self.selection_order.push(entry.path.clone());
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

    /// Points the panel at `path` with the cursor on the first row and no
    /// selection (the caller refreshes the listing).
    pub fn open_path(&mut self, path: PathBuf) {
        self.current_path = path;
        self.cursor_index = 0;
        self.clear_selection();
    }

    /// Starts computing the size of the listed folders among `paths`
    /// (symbolic links to folders and `..` are skipped).
    pub fn calculate_dir_sizes(&mut self, paths: &[PathBuf]) {
        let wanted: HashSet<&PathBuf> = paths.iter().collect();
        let folders = self
            .entries
            .iter()
            .filter(|e| e.is_dir && !e.is_symlink && e.name != ".." && wanted.contains(&e.path))
            .map(|e| e.path.clone())
            .collect();
        self.dir_sizes.request(folders, self.source.clone());
    }

    /// Folders for "calculate folder sizes": the targeted ones (selection or
    /// cursor), or every listed folder when none of them is a folder.
    pub fn calculate_targeted_dir_sizes(&mut self) {
        let targets = self.get_targeted_paths();
        let any_folder = self
            .entries
            .iter()
            .any(|e| e.is_dir && targets.contains(&e.path));
        let paths = if any_folder {
            targets
        } else {
            self.entries.iter().map(|e| e.path.clone()).collect()
        };
        self.calculate_dir_sizes(&paths);
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

    /// Files, folders (without `..`) and total file size of the listing.
    pub fn entry_totals(&self) -> (usize, usize, u64) {
        let files = self.entries.iter().filter(|e| !e.is_dir);
        let total_files = files.clone().count();
        let total_size = files.map(|e| e.size).sum();
        let total_dirs = self
            .entries
            .iter()
            .filter(|e| e.is_dir && e.name != "..")
            .count();
        (total_files, total_dirs, total_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A panel on `/tmp` listing `a.rs` and `b.rs`.
    fn panel_with_two_files() -> PanelState {
        let mut panel = PanelState::new(PathBuf::from("/tmp"));
        panel.entries = ["a.rs", "b.rs"]
            .into_iter()
            .map(|name| FileEntry {
                name: name.to_string(),
                path: PathBuf::from("/tmp").join(name),
                is_dir: false,
                is_symlink: false,
                size: 0,
                modified: None,
            })
            .collect();
        panel
    }

    #[test]
    fn test_invert_selection() {
        let mut panel = panel_with_two_files();
        panel.selected_paths.insert(PathBuf::from("/tmp/a.rs"));
        panel.invert_selection();
        assert!(!panel.selected_paths.contains(&PathBuf::from("/tmp/a.rs")));
        assert!(panel.selected_paths.contains(&PathBuf::from("/tmp/b.rs")));
    }

    #[test]
    fn test_selection_order() {
        let mut panel = panel_with_two_files();

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
