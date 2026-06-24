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
