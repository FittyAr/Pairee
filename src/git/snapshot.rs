//! Everything the Git panel shows, read in one go (runs on a background
//! thread; never call it from the render or input path).

use std::path::{Path, PathBuf};

/// Contents of the Git panel tabs.
#[derive(Debug, Clone, Default)]
pub struct RepoSnapshot {
    /// Root of the working directory.
    pub workdir: PathBuf,
    /// Checked-out branch, `None` when HEAD is detached.
    pub branch: Option<String>,
    pub status: Vec<crate::git::status::GitFileStatus>,
    pub log: Vec<crate::git::log::CommitInfo>,
    pub branches: Vec<crate::git::branches::BranchInfo>,
    pub stashes: Vec<crate::git::stash::StashInfo>,
    pub tags: Vec<crate::git::tags::TagInfo>,
}

/// Name of the checked-out branch (also for an unborn branch without
/// commits); `None` when HEAD is detached.
pub fn current_branch(repo: &git2::Repository) -> Option<String> {
    if repo.head_detached().unwrap_or(false) {
        return None;
    }
    if let Ok(head) = repo.head()
        && let Ok(name) = head.shorthand()
    {
        return Some(name.to_string());
    }
    let head_ref = repo.find_reference("HEAD").ok()?;
    let target = head_ref.symbolic_target().ok().flatten()?;
    target.strip_prefix("refs/heads/").map(str::to_string)
}

/// Reads the repository containing `path`, with up to `log_limit` commits.
/// `None` when `path` is not inside a repository.
pub fn load(path: &Path, log_limit: usize) -> Option<RepoSnapshot> {
    let mut repo = crate::git::repo::find_repo(path)?;
    Some(RepoSnapshot {
        workdir: crate::git::repo::get_workdir(&repo).unwrap_or_else(|| path.to_path_buf()),
        branch: current_branch(&repo),
        status: crate::git::status::get_status(&repo),
        log: crate::git::log::get_log(&repo, log_limit),
        branches: crate::git::branches::get_branches(&repo),
        stashes: crate::git::stash::list_stashes(&mut repo).unwrap_or_default(),
        tags: crate::git::tags::list_tags(&repo).unwrap_or_default(),
    })
}

/// The `limit` commits after the first `skip` ones (log paging).
pub fn load_log_page(path: &Path, skip: usize, limit: usize) -> Vec<crate::git::log::CommitInfo> {
    crate::git::repo::find_repo(path)
        .map(|repo| crate::git::log::get_log_paged(&repo, skip, limit))
        .unwrap_or_default()
}
