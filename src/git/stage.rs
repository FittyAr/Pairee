use std::path::Path;

/// Stages a single file (adds it to the index).
pub fn stage_file(repo: &git2::Repository, file_path: &str) -> anyhow::Result<()> {
    let mut index = repo.index()?;
    index.add_path(Path::new(file_path))?;
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
        // It's possible the path is not in index, ignore errors from remove_path
        let _ = index.remove_path(path);
        index.write()?;
    }
    Ok(())
}
