//! `git pull`: fetch, then fast-forward or merge the branch's upstream.

use crate::config::localization::t;
use crate::git::remote::{ProgressObserver, fetch};
use crate::git::signature::default_signature;
use crate::git::worktree::{fast_forward, safe_checkout};

/// Pulls the upstream of `branch_name` (the current branch) into it.
///
/// The upstream is the branch's configured `branch.<name>.remote` /
/// `branch.<name>.merge`; without one it falls back to the same-named branch
/// on `remote_name`. The working tree is updated in safe mode, so the pull
/// fails (and changes nothing) when it would overwrite local changes.
pub fn pull(
    repo: &git2::Repository,
    remote_name: &str,
    branch_name: &str,
    observer: ProgressObserver<'_>,
) -> anyhow::Result<()> {
    let (remote_name, upstream_ref) = upstream_of(repo, remote_name, branch_name);
    fetch(repo, &remote_name, observer)?;

    let remote_ref = repo.find_reference(&upstream_ref)?;
    let annotated_commit = repo.reference_to_annotated_commit(&remote_ref)?;
    let (analysis, _) = repo.merge_analysis(&[&annotated_commit])?;

    if analysis.is_up_to_date() {
        Ok(())
    } else if analysis.is_fast_forward() {
        fast_forward(repo, annotated_commit.id(), "pull: Fast-forward")
    } else if analysis.is_normal() {
        let upstream_name = remote_ref.shorthand().unwrap_or(&upstream_ref);
        let message = format!("Merge branch '{upstream_name}' into {branch_name}");
        merge_commit(repo, annotated_commit.id(), &message)
    } else {
        anyhow::bail!("Unsupported merge analysis result: {:?}", analysis)
    }
}

/// `(remote to fetch, upstream reference)` for `branch_name`.
fn upstream_of(repo: &git2::Repository, remote_name: &str, branch_name: &str) -> (String, String) {
    let local_ref = format!("refs/heads/{branch_name}");
    let configured = |buf: Result<git2::Buf, git2::Error>| {
        buf.ok().and_then(|b| b.as_str().ok().map(str::to_string))
    };
    match (
        configured(repo.branch_upstream_remote(&local_ref)),
        configured(repo.branch_upstream_name(&local_ref)),
    ) {
        (Some(remote), Some(upstream)) => (remote, upstream),
        _ => (
            remote_name.to_string(),
            format!("refs/remotes/{remote_name}/{branch_name}"),
        ),
    }
}

/// Merges `theirs` into HEAD and records a two-parent merge commit. Refuses
/// on conflicts and on local changes the merge result would overwrite.
fn merge_commit(repo: &git2::Repository, theirs: git2::Oid, message: &str) -> anyhow::Result<()> {
    let local_commit = repo.head()?.peel_to_commit()?;
    let remote_commit = repo.find_commit(theirs)?;

    let mut index = repo.merge_commits(&local_commit, &remote_commit, None)?;
    if index.has_conflicts() {
        anyhow::bail!(t("git_error_merge_conflicts"));
    }
    let tree = repo.find_tree(index.write_tree_to(repo)?)?;
    safe_checkout(repo, tree.as_object())?;

    let sig = default_signature(repo)?;
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &[&local_commit, &remote_commit],
    )?;
    Ok(())
}
