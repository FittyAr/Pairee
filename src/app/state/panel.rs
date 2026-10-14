use super::dir_sizes::DirSizes;
use super::refresh::listing::{ListingProgress, PanelListing};
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
    /// First entry shown. The view only scrolls when the cursor leaves it
    /// (as in Far and MC), so rows stay put under the mouse; kept up to
    /// date by the renderer, which knows the panel height.
    pub scroll_top: std::cell::Cell<usize>,
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
    pub listing: JobSlot<PanelListing, ListingProgress>,
    /// Branch view: every file below `current_path`, flat (Total
    /// Commander's Ctrl+B). Leaving the folder turns it off.
    pub branch_view: bool,
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
            scroll_top: Default::default(),
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
            branch_view: false,
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

    /// Files found so far by a branch view being read.
    pub fn branch_progress(&self) -> Option<ListingProgress> {
        self.branch_view.then(|| self.listing.progress()).flatten()
    }

    /// Points the panel at the folder holding `path`, with the cursor on it
    /// once the listing arrives (the caller refreshes the listing).
    pub fn reveal(&mut self, path: &std::path::Path) {
        let Some(parent) = path.parent() else {
            return;
        };
        self.open_path(parent.to_path_buf());
        self.pending_focus = Some(crate::fs::file_name_lossy(path));
    }

    /// Stops the listing and folder-size jobs (the tab is going away).
    pub fn cancel_jobs(&mut self) {
        self.listing.cancel();
        self.dir_sizes.cancel();
    }

    /// Moves the cursor index up by one, wrapping at boundaries.
    pub fn move_cursor_up(&mut self) {
        if !self.entries.is_empty() {
            self.cursor_index =
                crate::app::list_nav::wrap_prev(self.cursor_index, self.entries.len());
        }
    }

    /// Moves the cursor index down by one, wrapping at boundaries.
    pub fn move_cursor_down(&mut self) {
        if !self.entries.is_empty() {
            self.cursor_index =
                crate::app::list_nav::wrap_next(self.cursor_index, self.entries.len());
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

    /// Points the panel at `path` with the cursor on the first row and no
    /// selection (the caller refreshes the listing).
    pub fn open_path(&mut self, path: PathBuf) {
        self.current_path = path;
        self.cursor_index = 0;
        self.clear_selection();
    }

    /// [`Self::open_path`] for a local folder (drives, tree, histories,
    /// search results, frecency): a panel showing a server or an archive
    /// goes back to the local disk (an archive path is reopened as one).
    pub fn open_local(&mut self, path: PathBuf) {
        self.source = crate::fs::vfs::PanelSource::Local;
        self.open_path(path);
    }

    /// [`Self::open_path`] on the server of a new connection `conn`.
    pub fn open_remote(&mut self, conn: crate::fs::remote::RemoteConn, path: PathBuf) {
        self.source = crate::fs::vfs::PanelSource::Remote(conn);
        self.open_path(path);
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
