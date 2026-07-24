/// Merges the specified branch into the current HEAD.
///
/// For fast-forward merges, updates HEAD directly.
/// For normal merges, runs the merge process, updating the index and working directory,
/// and leaving the repository in a merging state for the user to commit or resolve conflicts.
pub fn merge(repo: &git2::Repository, branch_name: &str) -> anyhow::Result<git2::MergeAnalysis> {
    let branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
    let branch_ref = branch.get();
    let annotated_commit = repo.reference_to_annotated_commit(branch_ref)?;

    let (analysis, _) = repo.merge_analysis(&[&annotated_commit])?;

    if analysis.is_fast_forward() {
        let mut head_ref = repo.head()?;
        let msg = format!("merge {}: Fast-forward", branch_name);
        head_ref.set_target(annotated_commit.id(), &msg)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
    } else if analysis.is_normal() {
        repo.merge(&[&annotated_commit], None, None)?;
    }

    Ok(analysis)
}

/// Aborts an in-progress merge, reverting the working directory and index.
pub fn abort_merge(repo: &git2::Repository) -> anyhow::Result<()> {
    repo.cleanup_state()?;

    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    repo.reset(commit.as_object(), git2::ResetType::Hard, None)?;
    Ok(())
}
