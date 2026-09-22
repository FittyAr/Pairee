/// Extracts the local branch name from a remote branch name (e.g. "origin/feature" -> "feature").
pub fn local_branch_name_for_remote(repo: &git2::Repository, remote_branch_name: &str) -> String {
    if let Ok(remotes) = repo.remotes() {
        for remote in &remotes {
            if let Ok(Some(remote)) = remote {
                let prefix = format!("{}/", remote);
                if let Some(stripped) = remote_branch_name.strip_prefix(&prefix) {
                    return stripped.to_string();
                }
            }
        }
    }
    if let Some((_remote, local)) = remote_branch_name.split_once('/') {
        local.to_string()
    } else {
        remote_branch_name.to_string()
    }
}

/// Checks out a branch by name (local or remote).
///
/// If `branch_name` is an existing local branch, it performs a standard local checkout.
/// If `branch_name` is a remote-tracking branch (e.g. "origin/feat") or matches one,
/// it creates the local tracking branch if needed, configures its upstream, and checks it out.
///
/// Returns the name of the local branch that was checked out.
pub fn checkout_branch(repo: &git2::Repository, branch_name: &str) -> anyhow::Result<String> {
    // 1. Try finding as a local branch first
    if let Ok(local_branch) = repo.find_branch(branch_name, git2::BranchType::Local) {
        let branch_ref = local_branch.get();
        let branch_ref_name = branch_ref.name()?;
        let obj = repo.revparse_single(branch_ref_name)?;
        repo.checkout_tree(&obj, None)?;
        repo.set_head(branch_ref_name)?;
        return Ok(branch_name.to_string());
    }

    // 2. Try finding as a remote branch directly (e.g. "origin/feat")
    if let Ok(remote_branch) = repo.find_branch(branch_name, git2::BranchType::Remote) {
        let local_name = local_branch_name_for_remote(repo, branch_name);
        if local_name.is_empty() || local_name == "HEAD" {
            anyhow::bail!("Cannot checkout remote HEAD reference");
        }
        return checkout_remote_tracking(repo, &local_name, &remote_branch, branch_name);
    }

    // 3. Maybe branch_name is a local name that does not exist yet, but matches a remote branch
    if let Ok(remote_branches) = repo.branches(Some(git2::BranchType::Remote)) {
        for branch_res in remote_branches.flatten() {
            let (rem_branch, _) = branch_res;
            if let Ok(Some(rem_name)) = rem_branch.name()
                && !rem_name.ends_with("/HEAD")
                && local_branch_name_for_remote(repo, rem_name) == branch_name
            {
                return checkout_remote_tracking(repo, branch_name, &rem_branch, rem_name);
            }
        }
    }

    anyhow::bail!("Branch not found: {}", branch_name)
}

fn checkout_remote_tracking(
    repo: &git2::Repository,
    local_name: &str,
    remote_branch: &git2::Branch,
    remote_ref_name: &str,
) -> anyhow::Result<String> {
    // Check if local branch already exists
    if let Ok(mut local_branch) = repo.find_branch(local_name, git2::BranchType::Local) {
        if local_branch.upstream().is_err() {
            let _ = local_branch.set_upstream(Some(remote_ref_name));
        }
        let branch_ref = local_branch.get();
        let branch_ref_name = branch_ref.name()?;
        let obj = repo.revparse_single(branch_ref_name)?;
        repo.checkout_tree(&obj, None)?;
        repo.set_head(branch_ref_name)?;
        return Ok(local_name.to_string());
    }

    // Create new local branch pointing to remote branch commit
    let commit = remote_branch.get().peel_to_commit()?;
    let mut new_branch = repo.branch(local_name, &commit, false)?;

    // Set upstream tracking
    if let Err(e) = new_branch.set_upstream(Some(remote_ref_name)) {
        log::warn!("Failed to set upstream for branch {}: {}", local_name, e);
    }

    // Checkout the newly created branch
    let branch_ref = new_branch.get();
    let branch_ref_name = branch_ref.name()?;
    let obj = repo.revparse_single(branch_ref_name)?;
    repo.checkout_tree(&obj, None)?;
    repo.set_head(branch_ref_name)?;

    Ok(local_name.to_string())
}

/// Checks out a specific commit by its hash (full or short), leaving HEAD detached.
pub fn checkout_commit(repo: &git2::Repository, commit_hash: &str) -> anyhow::Result<()> {
    let obj = repo.revparse_single(commit_hash)?;
    repo.checkout_tree(&obj, None)?;
    repo.set_head_detached(obj.id())?;
    Ok(())
}
