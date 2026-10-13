//! Working-tree updates that never discard the user's local changes.

use crate::config::localization::t;

/// Checks out `target` in safe mode: files the user modified (or untracked
/// files in the way) that the checkout would overwrite make it fail with a
/// localized error instead of being clobbered. Unrelated local changes stay.
pub fn safe_checkout(repo: &git2::Repository, target: &git2::Object<'_>) -> anyhow::Result<()> {
    let mut opts = git2::build::CheckoutBuilder::new();
    opts.safe();
    repo.checkout_tree(target, Some(&mut opts))
        .map_err(local_changes_error)
}

/// Maps libgit2's "would overwrite local changes" conflict to a localized error.
pub fn local_changes_error(err: git2::Error) -> anyhow::Error {
    if err.code() == git2::ErrorCode::Conflict {
        anyhow::anyhow!(t("git_error_local_changes_overwritten"))
    } else {
        err.into()
    }
}

/// Fast-forwards HEAD (its branch, or HEAD itself when detached) to `target`
/// after safely updating the working tree.
pub fn fast_forward(
    repo: &git2::Repository,
    target: git2::Oid,
    reflog: &str,
) -> anyhow::Result<()> {
    let commit = repo.find_commit(target)?;
    safe_checkout(repo, commit.as_object())?;
    match repo.head() {
        Ok(mut head) if head.is_branch() => {
            head.set_target(target, reflog)?;
        }
        Ok(_) => repo.set_head_detached(target)?,
        // No commit yet: the branch HEAD names starts at `target`.
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
            let head = repo.find_reference("HEAD")?;
            let branch = head.symbolic_target().ok().flatten().unwrap_or("HEAD");
            repo.reference(branch, target, true, reflog)?;
        }
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
