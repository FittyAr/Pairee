use crate::git::snapshot::RepoSnapshot;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct GitPanelState {
    pub repo_path: PathBuf,
    pub active_tab: usize,
    pub cursor_idx: usize,
    pub scroll: usize,
    pub status_entries: Vec<crate::git::status::GitFileStatus>,
    pub log_entries: Vec<crate::git::log::CommitInfo>,
    pub branch_entries: Vec<crate::git::branches::BranchInfo>,
    pub stash_entries: Vec<crate::git::stash::StashInfo>,
    pub tag_entries: Vec<crate::git::tags::TagInfo>,
    pub current_branch: String,
    /// The whole history is loaded (no further log pages to fetch).
    pub log_complete: bool,
}

impl GitPanelState {
    /// An empty panel for `repo_path`, shown while its contents load.
    pub fn empty(repo_path: &Path, active_tab: usize, cursor_idx: usize) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
            active_tab,
            cursor_idx,
            scroll: 0,
            status_entries: Vec::new(),
            log_entries: Vec::new(),
            branch_entries: Vec::new(),
            stash_entries: Vec::new(),
            tag_entries: Vec::new(),
            current_branch: String::new(),
            log_complete: false,
        }
    }

    /// Number of rows in tab `tab`.
    pub fn tab_len(&self, tab: usize) -> usize {
        match tab {
            0 => self.status_entries.len(),
            1 => self.log_entries.len(),
            2 => self.branch_entries.len(),
            3 => self.stash_entries.len(),
            4 => self.tag_entries.len(),
            _ => 0,
        }
    }

    /// Replaces the contents with a freshly read `snapshot` (`log_limit`
    /// commits were requested) and keeps the cursor inside the list.
    pub fn apply_snapshot(&mut self, snapshot: RepoSnapshot, log_limit: usize, detached: String) {
        self.repo_path = snapshot.workdir;
        self.current_branch = snapshot.branch.unwrap_or(detached);
        self.log_complete = snapshot.log.len() < log_limit;
        self.status_entries = snapshot.status;
        self.log_entries = snapshot.log;
        self.branch_entries = snapshot.branches;
        self.stash_entries = snapshot.stashes;
        self.tag_entries = snapshot.tags;
        self.clamp_cursor();
    }

    /// Keeps the cursor on an existing row of the active tab.
    pub fn clamp_cursor(&mut self) {
        let len = self.tab_len(self.active_tab);
        self.cursor_idx = self.cursor_idx.min(len.saturating_sub(1));
        self.scroll = self.scroll.min(self.cursor_idx);
    }
}
