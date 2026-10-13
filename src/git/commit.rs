use crate::git::signature::{fallback_identity, resolve_signature};

/// File in the git directory listing the other parents of an in-progress merge.
const MERGE_HEAD_FILE: &str = "MERGE_HEAD";

/// Creates a commit with all staged changes.
///
/// If `author_name` or `author_email` are empty, they are read from the repo config.
pub fn commit(
    repo: &git2::Repository,
    message: &str,
    author_name: &str,
    author_email: &str,
) -> anyhow::Result<git2::Oid> {
    let sig = resolve_signature(repo, author_name, author_email, fallback_identity())?;
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let parent_commits = commit_parents(repo)?;
    let parents: Vec<&git2::Commit> = parent_commits.iter().collect();

    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;
    finish_pending_operation(repo)?;
    Ok(oid)
}

/// HEAD (if any) followed by every `MERGE_HEAD` of an in-progress merge, so
/// concluding a merge records all of its parents.
fn commit_parents(repo: &git2::Repository) -> anyhow::Result<Vec<git2::Commit<'_>>> {
    let mut parents: Vec<git2::Commit<'_>> = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .into_iter()
        .collect();
    if repo.state() == git2::RepositoryState::Merge {
        // `mergehead_foreach` needs `&mut Repository`; MERGE_HEAD is a plain
        // file with one commit id per line.
        let merge_heads = std::fs::read_to_string(repo.path().join(MERGE_HEAD_FILE))?;
        for line in merge_heads.lines().map(str::trim).filter(|l| !l.is_empty()) {
            parents.push(repo.find_commit(git2::Oid::from_str(line)?)?);
        }
    }
    Ok(parents)
}

/// Clears `MERGE_HEAD`/`CHERRY_PICK_HEAD`/`REVERT_HEAD` (and their message
/// files) once the commit that concludes the operation exists. Rebases keep
/// their own state, which the rebase machinery owns.
fn finish_pending_operation(repo: &git2::Repository) -> anyhow::Result<()> {
    use git2::RepositoryState as S;
    if matches!(
        repo.state(),
        S::Merge | S::CherryPick | S::CherryPickSequence | S::Revert | S::RevertSequence
    ) {
        repo.cleanup_state()?;
    }
    Ok(())
}

/// Amends the current HEAD commit with staged changes and/or a new message.
pub fn commit_amend(
    repo: &git2::Repository,
    message: &str,
    author_name: &str,
    author_email: &str,
) -> anyhow::Result<git2::Oid> {
    let head = repo.head()?;
    let parent = head.peel_to_commit()?;

    let (default_name, default_email) = fallback_identity();
    let parent_author = parent.author();
    let fallback = (
        parent_author.name().unwrap_or(default_name),
        parent_author.email().unwrap_or(default_email),
    );
    let sig = resolve_signature(repo, author_name, author_email, fallback)?;
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let msg = if message.trim().is_empty() {
        parent.message().unwrap_or("Amended commit")
    } else {
        message
    };

    let oid = parent.amend(
        Some("HEAD"),
        Some(&sig),
        Some(&sig),
        None,
        Some(msg),
        Some(&tree),
    )?;
    Ok(oid)
}
