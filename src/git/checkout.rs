/// Checks out a local branch by name.
///
/// Sets HEAD to the branch and updates the working tree to match.
pub fn checkout_branch(repo: &git2::Repository, branch_name: &str) -> anyhow::Result<()> {
    // Find the branch reference
    let branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
    let branch_ref = branch.get();
    let branch_ref_name = branch_ref
        .name()
        .ok_or_else(|| anyhow::anyhow!("Invalid branch ref name"))?;

    // Resolve the commit the branch points to
    let obj = repo.revparse_single(branch_ref_name)?;

    // Perform the checkout (update working tree)
    repo.checkout_tree(&obj, None)?;

    // Update HEAD to point to the branch
    repo.set_head(branch_ref_name)?;
    Ok(())
}

/// Checks out a specific commit by its hash (full or short), leaving HEAD detached.
pub fn checkout_commit(repo: &git2::Repository, commit_hash: &str) -> anyhow::Result<()> {
    let obj = repo.revparse_single(commit_hash)?;
    repo.checkout_tree(&obj, None)?;
    repo.set_head_detached(obj.id())?;
    Ok(())
}
