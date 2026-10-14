//! How an operation that combines histories ended (merge, pull,
//! cherry-pick, revert), and the "commit unless there are conflicts" step
//! they share. Like the Git CLI, a clean result is committed at once with
//! the default message; conflicts leave the operation in progress, with
//! conflict markers in the files, for the user to resolve, stage and
//! commit (or abort).

use std::collections::BTreeSet;
use std::path::PathBuf;

/// The end state of a merge-like operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing to do: HEAD already contains the other side.
    UpToDate,
    /// HEAD moved forward to the other side (no new commit).
    FastForward,
    /// The result was committed.
    Committed(git2::Oid),
    /// Stopped with these conflicted files (repository-relative, `/`);
    /// the operation is in progress (`MERGE_HEAD`, `CHERRY_PICK_HEAD`, ...).
    Conflicts(Vec<String>),
}

/// An operation the repository is in the middle of (waiting for the user
/// to resolve conflicts and commit, or to abort).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingOperation {
    Merge,
    CherryPick,
    Revert,
    Rebase,
}

impl PendingOperation {
    /// The operation `repo` is in the middle of, if any.
    pub fn of(repo: &git2::Repository) -> Option<Self> {
        use git2::RepositoryState as S;
        match repo.state() {
            S::Merge => Some(Self::Merge),
            S::CherryPick | S::CherryPickSequence => Some(Self::CherryPick),
            S::Revert | S::RevertSequence => Some(Self::Revert),
            S::Rebase | S::RebaseInteractive | S::RebaseMerge => Some(Self::Rebase),
            _ => None,
        }
    }

    /// Whether Pairee can abort it (merge, cherry-pick, revert).
    pub fn abortable(self) -> bool {
        !matches!(self, Self::Rebase)
    }
}

/// Refuses to start a merge, cherry-pick, revert, rebase or pull while
/// another one waits to be concluded: its state (`MERGE_HEAD`, ...) would
/// leak into the new operation's commit.
pub fn ensure_no_pending(repo: &git2::Repository) -> anyhow::Result<()> {
    if PendingOperation::of(repo).is_some() {
        anyhow::bail!(crate::config::localization::t(
            "git_error_operation_pending"
        ));
    }
    Ok(())
}

/// Refuses to start an operation that commits the index (a real merge,
/// cherry-pick, revert) while it holds staged changes, like Git: they would
/// be swept into its commit, and aborting it would discard them.
pub fn ensure_nothing_staged(repo: &git2::Repository) -> anyhow::Result<()> {
    let head_tree = match repo.head() {
        Ok(head) => Some(head.peel_to_tree()?),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => None,
        Err(e) => return Err(e.into()),
    };
    let index = repo.index()?;
    let diff = repo.diff_tree_to_index(head_tree.as_ref(), Some(&index), None)?;
    if diff.deltas().len() > 0 {
        anyhow::bail!(crate::config::localization::t("git_error_staged_changes"));
    }
    Ok(())
}

/// Both checks above, for an operation that always commits the index.
pub fn ensure_ready(repo: &git2::Repository) -> anyhow::Result<()> {
    ensure_no_pending(repo)?;
    ensure_nothing_staged(repo)
}

/// The repository-relative path of an index entry: its bytes as they are
/// on Unix (a name that is not UTF-8 stays usable), UTF-8 elsewhere (what
/// Git writes on Windows).
pub fn index_path(bytes: &[u8]) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(String::from_utf8_lossy(bytes).as_ref())
    }
}

/// Every path the conflicts of `index` involve (any side), sorted and
/// deduplicated.
pub fn conflict_paths(index: &git2::Index) -> anyhow::Result<BTreeSet<PathBuf>> {
    let mut paths = BTreeSet::new();
    for conflict in index.conflicts()? {
        let conflict = conflict?;
        for entry in [conflict.ancestor, conflict.our, conflict.their]
            .into_iter()
            .flatten()
        {
            paths.insert(index_path(&entry.path));
        }
    }
    Ok(paths)
}

/// The conflicted paths of the index as shown to the user (`/`
/// separated), sorted and deduplicated.
pub fn conflicted_paths(repo: &git2::Repository) -> anyhow::Result<Vec<String>> {
    Ok(conflict_paths(&repo.index()?)?
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect())
}

/// After the index and working tree hold the operation's result: reports
/// the conflicts, or commits the result with `message` (by `author`, or the
/// configured identity) and ends the operation.
pub fn commit_or_conflicts(
    repo: &git2::Repository,
    message: &str,
    author: Option<&git2::Signature<'_>>,
) -> anyhow::Result<Outcome> {
    let conflicts = conflicted_paths(repo)?;
    if !conflicts.is_empty() {
        return Ok(Outcome::Conflicts(conflicts));
    }
    let oid = crate::git::commit::commit_staged(repo, message, author)?;
    Ok(Outcome::Committed(oid))
}

/// The message Git prepared for the operation (`MERGE_MSG`), or `fallback`.
pub fn prepared_message(repo: &git2::Repository, fallback: &str) -> String {
    repo.message()
        .ok()
        .map(|m| m.trim_end().to_string())
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}
