/// A single branch entry for display.
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Branch name (e.g. "main", "origin/main")
    pub name: String,
    /// Whether this is the currently checked-out branch
    pub is_current: bool,
    /// Whether this is a remote-tracking branch
    pub is_remote: bool,
}

/// Returns all local and remote branches in the repository.
pub fn get_branches(repo: &git2::Repository) -> Vec<BranchInfo> {
    let mut result = Vec::new();

    // Determine the current HEAD branch name for marking
    let current_branch_name = repo.head().ok().and_then(|h| {
        if h.is_branch() {
            h.shorthand().ok().map(|s| s.to_string())
        } else {
            None
        }
    });

    let branch_types = [
        (git2::BranchType::Local, false),
        (git2::BranchType::Remote, true),
    ];

    for (branch_type, is_remote) in &branch_types {
        if let Ok(branches) = repo.branches(Some(*branch_type)) {
            for branch_res in branches.flatten() {
                let (branch, _) = branch_res;
                let name = match branch.name() {
                    Ok(Some(n)) => n.to_string(),
                    _ => continue,
                };

                let is_current = !is_remote
                    && current_branch_name
                        .as_deref()
                        .map(|cur| cur == name)
                        .unwrap_or(false);

                result.push(BranchInfo {
                    name,
                    is_current,
                    is_remote: *is_remote,
                });
            }
        }
    }

    // Sort: local first, remotes at the end; current branch first
    result.sort_by(|a, b| {
        if a.is_current {
            std::cmp::Ordering::Less
        } else if b.is_current {
            std::cmp::Ordering::Greater
        } else {
            a.is_remote.cmp(&b.is_remote).then(a.name.cmp(&b.name))
        }
    });

    result
}

/// Creates a new local branch starting at the specified point (commit, tag or branch name).
pub fn create_branch(repo: &git2::Repository, branch_name: &str, start_point: &str) -> anyhow::Result<()> {
    let obj = repo.revparse_single(start_point)?;
    let commit = obj.peel_to_commit()?;
    repo.branch(branch_name, &commit, false)?;
    Ok(())
}

/// Deletes a local branch by name.
pub fn delete_branch(repo: &git2::Repository, branch_name: &str) -> anyhow::Result<()> {
    let mut branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
    branch.delete()?;
    Ok(())
}

/// Renames a local branch.
pub fn rename_branch(repo: &git2::Repository, old_name: &str, new_name: &str) -> anyhow::Result<()> {
    let mut branch = repo.find_branch(old_name, git2::BranchType::Local)?;
    branch.rename(new_name, false)?;
    Ok(())
}
