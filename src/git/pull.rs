//! `git pull`: fetch, then fast-forward or merge the branch's upstream.

use crate::git::merge::merge_annotated;
use crate::git::outcome::Outcome;
use crate::git::remote::{NetHooks, fetch};

/// Pulls the upstream of `branch_name` (the current branch) into it.
///
/// The upstream is the branch's configured `branch.<name>.remote` /
/// `branch.<name>.merge`; without one it falls back to the same-named branch
/// on `remote_name`. The working tree is updated in safe mode, so the pull
/// fails (and changes nothing) when it would overwrite local changes. Like
/// `git pull`, a clean merge is committed and conflicts leave the merge in
/// progress, with conflict markers in the files ([`Outcome::Conflicts`]).
pub fn pull(
    repo: &git2::Repository,
    remote_name: &str,
    branch_name: &str,
    hooks: NetHooks<'_>,
) -> anyhow::Result<Outcome> {
    let (remote_name, upstream_ref) = upstream_of(repo, remote_name, branch_name);
    fetch(repo, &remote_name, hooks)?;

    let remote_ref = repo.find_reference(&upstream_ref)?;
    let annotated_commit = repo.reference_to_annotated_commit(&remote_ref)?;
    let upstream_name = remote_ref.shorthand().unwrap_or(&upstream_ref);
    let message = format!("Merge branch '{upstream_name}' into {branch_name}");
    merge_annotated(repo, &annotated_commit, upstream_name, Some(&message))
}

/// `(remote to fetch, upstream reference)` for `branch_name`.
fn upstream_of(repo: &git2::Repository, remote_name: &str, branch_name: &str) -> (String, String) {
    let local_ref = format!("refs/heads/{branch_name}");
    use crate::git::remote::config_value as configured;
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
