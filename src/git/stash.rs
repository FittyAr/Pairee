/// Information about a single stash entry.
#[derive(Debug, Clone)]
pub struct StashInfo {
    /// Index in the stash stack (0 is the most recent)
    pub index: usize,
    /// Message associated with the stash entry
    pub message: String,
    /// The full commit hash of the stash commit
    pub oid: String,
}

/// Retrieves the list of stashes in the repository.
pub fn list_stashes(repo: &mut git2::Repository) -> anyhow::Result<Vec<StashInfo>> {
    let mut list = Vec::new();
    repo.stash_foreach(|index, message, oid| {
        list.push(StashInfo {
            index,
            message: message.to_string(),
            oid: oid.to_string(),
        });
        true
    })?;
    Ok(list)
}

/// Saves the working directory and index changes to a new stash entry.
pub fn stash_save(
    repo: &mut git2::Repository,
    message: Option<&str>,
    include_untracked: bool,
) -> anyhow::Result<()> {
    let sig = repo.signature().unwrap_or_else(|_| {
        git2::Signature::now("Pairee User", "pairee@localhost").unwrap()
    });
    let mut flags = git2::StashFlags::DEFAULT;
    if include_untracked {
        flags.insert(git2::StashFlags::INCLUDE_UNTRACKED);
    }
    repo.stash_save(&sig, message.unwrap_or(""), Some(flags))?;
    Ok(())
}

/// Applies a stash entry by its index.
pub fn stash_apply(repo: &mut git2::Repository, index: usize) -> anyhow::Result<()> {
    let mut opts = git2::StashApplyOptions::new();
    repo.stash_apply(index, Some(&mut opts))?;
    Ok(())
}

/// Drops a stash entry by its index.
pub fn stash_drop(repo: &mut git2::Repository, index: usize) -> anyhow::Result<()> {
    repo.stash_drop(index)?;
    Ok(())
}

/// Applies a stash entry and drops it if application succeeds.
pub fn stash_pop(repo: &mut git2::Repository, index: usize) -> anyhow::Result<()> {
    let mut opts = git2::StashApplyOptions::new();
    repo.stash_apply(index, Some(&mut opts))?;
    repo.stash_drop(index)?;
    Ok(())
}
