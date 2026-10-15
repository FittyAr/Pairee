use crate::config::localization::t;
use crate::git::outcome::{Outcome, PendingOperation, commit_or_conflicts, prepared_message};
use crate::git::worktree::{fast_forward, local_changes_error};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// Merges the specified branch into the current HEAD, like `git merge`.
///
/// Fast-forwards move HEAD after a safe working-tree update. Normal merges
/// update the index and working directory; a clean result is committed at
/// once with Git's default message, conflicts leave the merge in progress
/// (`MERGE_HEAD`, conflict markers in the files) for the user to resolve and
/// commit or abort. Both refuse when local changes would be overwritten.
pub fn merge(repo: &git2::Repository, branch_name: &str) -> anyhow::Result<Outcome> {
    let branch = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .or_else(|_| repo.find_branch(branch_name, git2::BranchType::Remote))
        .map_err(|_| anyhow::anyhow!(t("git_error_branch_not_found").replace("{}", branch_name)))?;
    let annotated_commit = repo.reference_to_annotated_commit(branch.get())?;
    merge_annotated(repo, &annotated_commit, branch_name, None)
}

/// Merges `theirs` (the branch `label`) into HEAD. The merge commit gets
/// `message`, or the one Git prepared (`MERGE_MSG`, else "Merge branch").
pub fn merge_annotated(
    repo: &git2::Repository,
    theirs: &git2::AnnotatedCommit<'_>,
    label: &str,
    message: Option<&str>,
) -> anyhow::Result<Outcome> {
    crate::git::outcome::ensure_no_pending(repo)?;
    let (analysis, _) = repo.merge_analysis(&[theirs])?;
    if analysis.is_up_to_date() {
        Ok(Outcome::UpToDate)
    } else if analysis.is_fast_forward() || analysis.is_unborn() {
        fast_forward(repo, theirs.id(), &format!("merge {label}: Fast-forward"))?;
        Ok(Outcome::FastForward)
    } else if analysis.is_normal() {
        crate::git::outcome::ensure_nothing_staged(repo)?;
        repo.merge(&[theirs], None, None)
            .map_err(local_changes_error)?;
        let message = message
            .map(str::to_string)
            .unwrap_or_else(|| prepared_message(repo, &format!("Merge branch '{label}'")));
        commit_or_conflicts(repo, &message, None)
    } else {
        anyhow::bail!(t("git_error_merge_unsupported"))
    }
}

/// Aborts an in-progress merge, cherry-pick or revert like
/// `git merge --abort`: every path the operation touched (staged results
/// and conflicts) goes back to HEAD, while local changes the user had in
/// other files before it are kept.
pub fn abort_merge(repo: &git2::Repository) -> anyhow::Result<()> {
    // Without a merge/cherry-pick/revert to abort this would reset the index
    // and delete files for nothing; a rebase has its own state to unwind.
    if !PendingOperation::of(repo).is_some_and(PendingOperation::abortable) {
        anyhow::bail!(t("git_error_nothing_to_abort"));
    }
    let head = repo.head()?.peel_to_commit()?;
    let head_tree = head.tree()?;
    let touched = merge_touched_paths(repo, &head_tree)?;

    let workdir = crate::git::repo::workdir(repo)?.to_path_buf();
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
    paths.extend(crate::git::outcome::conflict_paths(&index)?);
    Ok(paths)
}
