use std::path::PathBuf;

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
}
