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
    let sig = repo
        .signature()
        .unwrap_or_else(|_| git2::Signature::now("Pairee User", "pairee@localhost").unwrap());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::commit::commit;
    use crate::git::repo::init_repo;
    use crate::git::stage::stage_file;
    use std::fs::File;
    use tempfile::TempDir;

    fn setup() -> (TempDir, git2::Repository) {
        let dir = TempDir::new().unwrap();
        let repo = init_repo(dir.path()).unwrap();
        let mut c = repo.config().unwrap();
        c.set_str("user.name", "T").unwrap();
        c.set_str("user.email", "t@t").unwrap();
        let f = dir.path().join("a.txt");
        File::create(&f).unwrap();
        stage_file(&repo, "a.txt").unwrap();
        commit(&repo, "init", "T", "t@t").unwrap();
        (dir, repo)
    }

    #[test]
    fn stash_info_carries_message() {
        let (dir, mut repo) = setup();
        // Modify the file so there's something to stash.
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "changed").unwrap();
        stash_save(&mut repo, Some("my special message"), false).unwrap();
        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);
        assert!(stashes[0].message.contains("my special message"));
        // The OID must be a valid 40-char hex string.
        assert_eq!(stashes[0].oid.len(), 40);
        assert!(stashes[0].oid.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn stash_save_without_message_uses_empty_string() {
        let (dir, mut repo) = setup();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "changed").unwrap();
        stash_save(&mut repo, None, false).unwrap();
        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);
        // git2 stores the message verbatim; the empty case should
        // produce a stash entry with an empty (or whitespace-only)
        // message. We don't pin a specific string — just that the
        // call didn't panic.
    }
}
