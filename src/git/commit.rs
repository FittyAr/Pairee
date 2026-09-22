/// Creates a commit with all staged changes.
///
/// If `author_name` or `author_email` are empty, they are read from the repo config.
pub fn commit(
    repo: &git2::Repository,
    message: &str,
    author_name: &str,
    author_email: &str,
) -> anyhow::Result<git2::Oid> {
    // Resolve author identity: prefer provided values, fall back to repo/global config
    let config = repo.config().ok();
    let resolved_name = if !author_name.is_empty() {
        author_name.to_string()
    } else {
        config
            .as_ref()
            .and_then(|c| c.get_string("user.name").ok())
            .unwrap_or_else(|| "Pairee User".to_string())
    };
    let resolved_email = if !author_email.is_empty() {
        author_email.to_string()
    } else {
        config
            .as_ref()
            .and_then(|c| c.get_string("user.email").ok())
            .unwrap_or_else(|| "pairee@localhost".to_string())
    };

    let sig = git2::Signature::now(&resolved_name, &resolved_email)?;
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

    let config = repo.config().ok();
    let resolved_name = if !author_name.is_empty() {
        author_name.to_string()
    } else {
        config
            .as_ref()
            .and_then(|c| c.get_string("user.name").ok())
            .unwrap_or_else(|| parent.author().name().unwrap_or("Pairee User").to_string())
    };
    let resolved_email = if !author_email.is_empty() {
        author_email.to_string()
    } else {
        config
            .as_ref()
            .and_then(|c| c.get_string("user.email").ok())
            .unwrap_or_else(|| {
                parent
                    .author()
                    .email()
                    .unwrap_or("pairee@localhost")
                    .to_string()
            })
    };

    let sig = git2::Signature::now(&resolved_name, &resolved_email)?;
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
