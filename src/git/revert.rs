use crate::git::outcome::{Outcome, commit_or_conflicts, ensure_ready, prepared_message};

/// Reverts a commit on the current branch, like `git revert`: a clean
/// result is committed with Git's "Revert ..." message; conflicts leave the
/// revert in progress for the user to resolve and commit (or abort).
pub fn revert(repo: &git2::Repository, commit_hash: &str) -> anyhow::Result<Outcome> {
    let commit = repo.revparse_single(commit_hash)?.peel_to_commit()?;
    ensure_ready(repo)?;
    repo.revert(&commit, None)
        .map_err(crate::git::worktree::local_changes_error)?;
    let fallback = format!(
        "Revert \"{}\"\n\nThis reverts commit {}.",
        commit.summary().ok().flatten().unwrap_or_default(),
        commit.id()
    );
    let message = prepared_message(repo, &fallback);
    commit_or_conflicts(repo, &message, None)
}
