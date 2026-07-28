/// Checks out a local branch by name.
///
/// Sets HEAD to the branch and updates the working tree to match.
pub fn checkout_branch(repo: &git2::Repository, branch_name: &str) -> anyhow::Result<()> {
    // Find the branch reference
    let branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
    let branch_ref = branch.get();
    let branch_ref_name = branch_ref.name()?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::branches::create_branch;
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
    fn checkout_branch_moves_head() {
        let (_dir, repo) = setup();
        create_branch(&repo, "feature", "HEAD").unwrap();
        checkout_branch(&repo, "feature").unwrap();
        // After checkout, HEAD is on `feature`. `shorthand` returns
        // a `Result<&str, _>` so we unwrap the success path.
        let head = repo.head().unwrap();
        let shorthand = head.shorthand().expect("branch shorthand");
        assert_eq!(shorthand, "feature");
    }

    #[test]
    fn checkout_nonexistent_branch_returns_error() {
        let (_dir, repo) = setup();
        let res = checkout_branch(&repo, "does-not-exist");
        assert!(res.is_err());
    }

    #[test]
    fn checkout_commit_detaches_head() {
        let (_dir, repo) = setup();
        let head_oid = repo.head().unwrap().peel_to_commit().unwrap().id();
        checkout_commit(&repo, &head_oid.to_string()).unwrap();
        // After a commit checkout, HEAD is detached (not on a branch).
        let head = repo.head().unwrap();
        assert!(
            !head.is_branch(),
            "HEAD should be detached after checkout_commit"
        );
    }
}
