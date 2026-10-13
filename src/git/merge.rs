use crate::git::worktree::{fast_forward, local_changes_error};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// Merges the specified branch into the current HEAD.
///
/// Fast-forwards move HEAD after a safe working-tree update. Normal merges
/// update the index and working directory and leave the merge in progress
/// (`MERGE_HEAD`) so the next commit records both parents. Both refuse when
/// local changes would be overwritten.
pub fn merge(repo: &git2::Repository, branch_name: &str) -> anyhow::Result<git2::MergeAnalysis> {
    let branch = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .or_else(|_| repo.find_branch(branch_name, git2::BranchType::Remote))?;
    let annotated_commit = repo.reference_to_annotated_commit(branch.get())?;

    let (analysis, _) = repo.merge_analysis(&[&annotated_commit])?;

    if analysis.is_fast_forward() {
        let msg = format!("merge {}: Fast-forward", branch_name);
        fast_forward(repo, annotated_commit.id(), &msg)?;
    } else if analysis.is_normal() {
        repo.merge(&[&annotated_commit], None, None)
            .map_err(local_changes_error)?;
    }

    Ok(analysis)
}

/// Aborts an in-progress merge like `git merge --abort`: every path the
/// merge touched (staged results and conflicts) goes back to HEAD, while
/// local changes the user had in other files before the merge are kept.
pub fn abort_merge(repo: &git2::Repository) -> anyhow::Result<()> {
    let head = repo.head()?.peel_to_commit()?;
    let head_tree = head.tree()?;
    let touched = merge_touched_paths(repo, &head_tree)?;

    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("No working directory"))?
        .to_path_buf();
    let mut restore = git2::build::CheckoutBuilder::new();
    restore.force().disable_pathspec_match(true);
    let mut any_restore = false;
    for path in &touched {
        if head_tree.get_path(path).is_ok() {
            restore.path(path);
            any_restore = true;
        } else {
            // Added by the merge: not part of HEAD, so it goes away.
            let full = workdir.join(path);
            if full.exists() {
                std::fs::remove_file(full)?;
            }
        }
    }
    if any_restore {
        repo.checkout_tree(head_tree.as_object(), Some(&mut restore))?;
    }

    repo.reset(head.as_object(), git2::ResetType::Mixed, None)?;
    repo.cleanup_state()?;
    Ok(())
}

/// Paths whose index entry differs from HEAD, plus conflicted paths.
fn merge_touched_paths(
    repo: &git2::Repository,
    head_tree: &git2::Tree<'_>,
) -> anyhow::Result<BTreeSet<PathBuf>> {
    let index = repo.index()?;
    let mut paths = BTreeSet::new();
    let diff = repo.diff_tree_to_index(Some(head_tree), Some(&index), None)?;
    for delta in diff.deltas() {
        for file in [delta.old_file(), delta.new_file()] {
            if let Some(path) = file.path() {
                paths.insert(path.to_path_buf());
            }
        }
    }
    for conflict in index.conflicts()? {
        let conflict = conflict?;
        for entry in [conflict.ancestor, conflict.our, conflict.their]
            .into_iter()
            .flatten()
        {
            paths.insert(PathBuf::from(String::from_utf8_lossy(&entry.path).as_ref()));
        }
    }
    Ok(paths)
}
