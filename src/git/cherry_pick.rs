use crate::git::outcome::{Outcome, commit_or_conflicts, prepared_message};

/// Cherry-picks a commit onto the current branch, like `git cherry-pick`:
/// a clean result is committed with the original message and author;
/// conflicts leave the cherry-pick in progress for the user to resolve and
/// commit (or abort).
pub fn cherry_pick(repo: &git2::Repository, commit_hash: &str) -> anyhow::Result<Outcome> {
    let commit = repo.revparse_single(commit_hash)?.peel_to_commit()?;
    repo.cherrypick(&commit, None)
        .map_err(crate::git::worktree::local_changes_error)?;
    let message = prepared_message(repo, commit.message().unwrap_or_default());
    commit_or_conflicts(repo, &message, Some(&commit.author()))
}
