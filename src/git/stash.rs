use crate::config::localization::t;

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
    let sig = crate::git::signature::default_signature(repo)?;
    let mut flags = git2::StashFlags::DEFAULT;
    if include_untracked {
        flags.insert(git2::StashFlags::INCLUDE_UNTRACKED);
    }
    repo.stash_save(&sig, message.unwrap_or(""), Some(flags))?;
    Ok(())
}

/// The current position of the stash `oid` (the list shown may be stale:
/// stashes made or dropped elsewhere shift the indexes).
fn stash_index(repo: &mut git2::Repository, oid: &str) -> anyhow::Result<usize> {
    list_stashes(repo)?
        .into_iter()
        .find(|s| s.oid == oid)
        .map(|s| s.index)
        .ok_or_else(|| anyhow::anyhow!(t("git_error_stash_gone")))
}

/// Applies the stash `oid`; fails, keeping the result, when applying left
/// conflicts.
pub fn stash_apply(repo: &mut git2::Repository, oid: &str) -> anyhow::Result<()> {
    let index = stash_index(repo, oid)?;
    let mut opts = git2::StashApplyOptions::new();
    repo.stash_apply(index, Some(&mut opts))?;
    let conflicts = crate::git::outcome::conflicted_paths(repo)?;
    if !conflicts.is_empty() {
        anyhow::bail!(t("git_error_stash_conflicts").replace("{}", &conflicts.join(", ")));
    }
    Ok(())
}

/// Drops the stash `oid`.
pub fn stash_drop(repo: &mut git2::Repository, oid: &str) -> anyhow::Result<()> {
    let index = stash_index(repo, oid)?;
    repo.stash_drop(index)?;
    Ok(())
}

/// Applies the stash `oid` and drops it, like `git stash pop`: a stash
/// whose application left conflicts is kept.
pub fn stash_pop(repo: &mut git2::Repository, oid: &str) -> anyhow::Result<()> {
    stash_apply(repo, oid)?;
    stash_drop(repo, oid)
}

/// Clears all stash entries.
pub fn stash_clear(repo: &mut git2::Repository) -> anyhow::Result<()> {
    if let Ok(mut reference) = repo.find_reference("refs/stash") {
        reference.delete()?;
    }
    Ok(())
}
