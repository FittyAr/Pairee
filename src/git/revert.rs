/// Reverts a commit on the current branch.
pub fn revert(repo: &git2::Repository, commit_hash: &str) -> anyhow::Result<()> {
    let obj = repo.revparse_single(commit_hash)?;
    let commit = obj.peel_to_commit()?;
    repo.revert(&commit, None)?;
    Ok(())
}
