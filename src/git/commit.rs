use crate::git::signature::{fallback_identity, resolve_signature};

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

    // Find parent commit (HEAD), if any
    let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;
    Ok(oid)
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
