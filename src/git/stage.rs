use std::path::Path;

/// Stages a single file (adds it to the index).
pub fn stage_file(repo: &git2::Repository, file_path: &str) -> anyhow::Result<()> {
    let mut index = repo.index()?;
    let path = Path::new(file_path);
    let workdir = crate::git::repo::workdir(repo)?;
    // `symlink_metadata`: a dangling symlink is still a file on disk.
    if workdir.join(path).symlink_metadata().is_ok() {
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
    // Index edits rather than `reset_default`, whose pathspec would treat
    // a name such as `[id].tsx` as a glob and unstage its siblings too.
    let path = Path::new(file_path);
    let mut index = repo.index()?;
    let head_entry = match repo.head() {
        Ok(head) => head.peel_to_tree()?.get_path(path).ok(),
        // Empty repository: nothing to restore.
        Err(_) => None,
    };
    match head_entry {
        Some(entry) => index.add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: entry.filemode() as u32,
            uid: 0,
            gid: 0,
            file_size: 0,
            id: entry.id(),
            flags: 0,
            flags_extended: 0,
            path: file_path.as_bytes().to_vec(),
        })?,
        None => {
            let _ = index.remove_path(path);
        }
    }
    index.write()?;
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

    let workdir = crate::git::repo::workdir(repo)?;
    let full_path = workdir.join(file_path);

    // If file exists in HEAD, check it out to restore working copy
    if let Ok(head_ref) = repo.head() {
        let commit = head_ref.peel_to_commit()?;
        let tree = commit.tree()?;
        if tree.get_path(Path::new(file_path)).is_ok() {
            let mut opts = git2::build::CheckoutBuilder::new();
            opts.force().disable_pathspec_match(true).path(file_path);
            repo.checkout_head(Some(&mut opts))?;
            return Ok(());
        }
    }

    // Otherwise, if it was an untracked or newly created file, delete it from disk
    if full_path.is_file() {
        std::fs::remove_file(&full_path)?;
    } else if full_path.is_dir() {
        // Git lists an untracked nested repository as one directory entry;
        // deleting it would lose its history.
        if full_path.join(".git").symlink_metadata().is_ok() {
            anyhow::bail!(crate::config::localization::t(
                "git_error_discard_nested_repo"
            ));
        }
        std::fs::remove_dir_all(&full_path)?;
    }

    Ok(())
}
