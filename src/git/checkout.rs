/// Extracts the local branch name from a remote branch name (e.g. "origin/feature" -> "feature").
pub fn local_branch_name_for_remote(repo: &git2::Repository, remote_branch_name: &str) -> String {
    crate::git::remote::split_remote_branch(repo, remote_branch_name)
        .map_or(remote_branch_name, |(_, local)| local)
        .to_string()
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
        switch_to_branch(repo, &local_branch)?;
        return Ok(branch_name.to_string());
    }

    // 2. Try finding as a remote branch directly (e.g. "origin/feat")
    if let Ok(remote_branch) = repo.find_branch(branch_name, git2::BranchType::Remote) {
        let local_name = local_branch_name_for_remote(repo, branch_name);
        if local_name.is_empty() || local_name == "HEAD" {
            anyhow::bail!(crate::config::localization::t(
                "git_error_checkout_remote_head"
            ));
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

    anyhow::bail!(
        crate::config::localization::t("git_error_branch_not_found").replace("{}", branch_name)
    )
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
        switch_to_branch(repo, &local_branch)?;
        return Ok(local_name.to_string());
    }

    // Create new local branch pointing to remote branch commit
    let commit = remote_branch.get().peel_to_commit()?;
    let mut new_branch = repo.branch(local_name, &commit, false)?;

    // Set upstream tracking
    if let Err(e) = new_branch.set_upstream(Some(remote_ref_name)) {
        log::warn!("Failed to set upstream for branch {}: {}", local_name, e);
    }

    switch_to_branch(repo, &new_branch)?;
    Ok(local_name.to_string())
}

/// Checks out a local branch's tree (safe mode: local changes that would be
/// overwritten make it fail) and points HEAD at the branch.
fn switch_to_branch(repo: &git2::Repository, branch: &git2::Branch) -> anyhow::Result<()> {
    let branch_ref_name = branch.get().name()?;
    let obj = repo.revparse_single(branch_ref_name)?;
    crate::git::worktree::safe_checkout(repo, &obj)?;
    repo.set_head(branch_ref_name)?;
    Ok(())
}

/// Checks out a specific commit by its hash (full or short), leaving HEAD detached.
pub fn checkout_commit(repo: &git2::Repository, commit_hash: &str) -> anyhow::Result<()> {
    let obj = repo
        .revparse_single(commit_hash)?
        .peel(git2::ObjectType::Commit)?;
    crate::git::worktree::safe_checkout(repo, &obj)?;
    repo.set_head_detached(obj.id())?;
    Ok(())
}
