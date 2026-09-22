use std::path::Path;

/// Stages a single file (adds it to the index).
pub fn stage_file(repo: &git2::Repository, file_path: &str) -> anyhow::Result<()> {
    let mut index = repo.index()?;
    let path = Path::new(file_path);
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("No working directory"))?;
    if workdir.join(path).exists() {
        index.add_path(path)?;
    } else {
        // File was deleted in working tree, stage the deletion
        let _ = index.remove_path(path);
    }
    index.write()?;
    Ok(())
}

/// Stages all changes in the working tree (equivalent to `git add -A`).
pub fn stage_all(repo: &git2::Repository) -> anyhow::Result<()> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.update_all(["*"].iter(), None)?;
    index.write()?;
    Ok(())
}

/// Unstages a single file (removes it from the index, matching HEAD or removing it if no commits exist).
pub fn unstage_file(repo: &git2::Repository, file_path: &str) -> anyhow::Result<()> {
    if let Ok(head_ref) = repo.head() {
        let commit = head_ref.peel_to_commit()?;
        let obj = commit.into_object();
        repo.reset_default(Some(&obj), Some(file_path))?;
    } else {
        // Empty repository, just remove the path from index if it is there
        let mut index = repo.index()?;
        let path = Path::new(file_path);
        let _ = index.remove_path(path);
        index.write()?;
    }
    Ok(())
}

/// Unstages all files in the index (matches HEAD or clears index if empty).
pub fn unstage_all(repo: &git2::Repository) -> anyhow::Result<()> {
    if let Ok(head_ref) = repo.head() {
        let commit = head_ref.peel_to_commit()?;
        repo.reset(commit.as_object(), git2::ResetType::Mixed, None)?;
    } else {
        let mut index = repo.index()?;
        index.clear()?;
        index.write()?;
    }
    Ok(())
}

/// Discards changes in a single file in the working tree and unstages it if staged.
pub fn discard_file_changes(repo: &git2::Repository, file_path: &str) -> anyhow::Result<()> {
    // First unstage any staged changes for this file
    let _ = unstage_file(repo, file_path);

    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("No working directory"))?;
    let full_path = workdir.join(file_path);

    // If file exists in HEAD, check it out to restore working copy
    if let Ok(head_ref) = repo.head() {
        let commit = head_ref.peel_to_commit()?;
        let tree = commit.tree()?;
        if tree.get_path(Path::new(file_path)).is_ok() {
            let mut opts = git2::build::CheckoutBuilder::new();
            opts.force().path(file_path);
            repo.checkout_head(Some(&mut opts))?;
            return Ok(());
        }
    }

    // Otherwise, if it was an untracked or newly created file, delete it from disk
    if full_path.is_file() {
        std::fs::remove_file(&full_path)?;
    } else if full_path.is_dir() {
        std::fs::remove_dir_all(&full_path)?;
    }

    Ok(())
}
