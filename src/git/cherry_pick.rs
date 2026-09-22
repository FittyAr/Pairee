/// Cherry-picks a commit onto the current branch.
pub fn cherry_pick(repo: &git2::Repository, commit_hash: &str) -> anyhow::Result<()> {
    let obj = repo.revparse_single(commit_hash)?;
    let commit = obj.peel_to_commit()?;
    repo.cherrypick(&commit, None)?;
    Ok(())
}
