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

/// Where the staged rename ending at `file_path` came from, if it is one.
/// A rename is a single status entry, but it is two index changes.
fn staged_rename_source(repo: &git2::Repository, file_path: &str) -> Option<String> {
    let head_tree = repo.head().ok()?.peel_to_tree().ok()?;
    let mut diff = repo
        .diff_tree_to_index(Some(&head_tree), Some(&repo.index().ok()?), None)
        .ok()?;
    diff.find_similar(None).ok()?;
    diff.deltas()
        .find(|d| {
            d.status() == git2::Delta::Renamed && d.new_file().path() == Some(Path::new(file_path))
        })
        .and_then(|d| d.old_file().path()?.to_str().map(str::to_owned))
}

/// Unstages a single file (removes it from the index, matching HEAD or removing it if no commits exist).
pub fn unstage_file(repo: &git2::Repository, file_path: &str) -> anyhow::Result<()> {
    if let Some(source) = staged_rename_source(repo, file_path) {
        unstage_path(repo, &source)?;
    }
    unstage_path(repo, file_path)
}

fn unstage_path(repo: &git2::Repository, file_path: &str) -> anyhow::Result<()> {
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
    let rename_source = staged_rename_source(repo, file_path);
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
    // A staged rename's old file comes back too.
    if let Some(source) = rename_source {
        let mut opts = git2::build::CheckoutBuilder::new();
        opts.force().disable_pathspec_match(true).path(&source);
        repo.checkout_head(Some(&mut opts))?;
    }

    // Otherwise, if it was an untracked or newly created file, delete it from disk
    // `symlink_metadata`: a symlink (even a dangling one) is removed as the
    // link it is, never followed.
    let kind = full_path.symlink_metadata().ok().map(|m| m.file_type());
    if kind.is_some_and(|k| k.is_file() || k.is_symlink()) {
        std::fs::remove_file(&full_path)?;
    } else if kind.is_some_and(|k| k.is_dir()) {
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
