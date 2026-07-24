/// Mapped enum for Git reset modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMode {
    /// Keeps changes in index and working directory
    Soft,
    /// Discards changes in index, keeps changes in working directory
    Mixed,
    /// Discards all changes in index and working directory
    Hard,
}

/// Resets the current HEAD to the specified target commit using the selected mode.
pub fn reset(repo: &git2::Repository, target_commit: &str, mode: ResetMode) -> anyhow::Result<()> {
    let obj = repo.revparse_single(target_commit)?;
    let commit = obj.peel_to_commit()?;
    let reset_type = match mode {
        ResetMode::Soft => git2::ResetType::Soft,
        ResetMode::Mixed => git2::ResetType::Mixed,
        ResetMode::Hard => git2::ResetType::Hard,
    };
    repo.reset(commit.as_object(), reset_type, None)?;
    Ok(())
}
