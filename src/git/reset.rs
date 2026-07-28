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
    fn reset_soft_keeps_working_tree_changes() {
        let (dir, repo) = setup();
        // Modify a.txt after the initial commit.
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "modified").unwrap();
        // Soft reset to HEAD: working tree unchanged, index unchanged,
        // only HEAD moves (which is already at HEAD for this case).
        let head = repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
            .to_string();
        let res = reset(&repo, &head, ResetMode::Soft);
        assert!(res.is_ok());
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "modified");
    }

    #[test]
    fn reset_hard_to_old_commit_discards_changes() {
        let (dir, repo) = setup();
        // Make a second commit, then hard-reset back to the first.
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "v2").unwrap();
        stage_file(&repo, "a.txt").unwrap();
        let _second = commit(&repo, "v2", "T", "t@t").unwrap();

        // First commit's OID: walk back via HEAD.parent.
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let first = head.parent(0).unwrap().id().to_string();

        reset(&repo, &first, ResetMode::Hard).unwrap();

        // Working tree is reverted to the first commit's content.
        let content = std::fs::read_to_string(&f).unwrap_or_default();
        assert!(
            content.is_empty(),
            "hard reset should revert to empty file, got: {content}"
        );
    }

    #[test]
    fn reset_to_invalid_target_returns_error() {
        let (_dir, repo) = setup();
        // "does-not-exist" cannot be resolved to a commit.
        let res = reset(&repo, "does-not-exist", ResetMode::Soft);
        assert!(res.is_err());
    }
}
