//! Push operations. Every push goes through [`push_refspecs`], which turns
//! per-reference rejections reported by the remote into errors.

use crate::config::localization::t;
use crate::git::remote::{NetHooks, create_callbacks};
use std::cell::RefCell;

/// Pushes `refspecs` to `remote`, failing when the remote rejects any of
/// them (non-fast-forward, protected branch, hook refusal, ...): libgit2
/// reports those through `push_update_reference`, not as a push error.
pub fn push_refspecs(
    remote: &mut git2::Remote<'_>,
    refspecs: &[String],
    hooks: NetHooks<'_>,
) -> anyhow::Result<()> {
    let rejected = RefCell::new(Vec::new());
    {
        let mut callbacks = create_callbacks(hooks);
        callbacks.push_update_reference(|refname, status| {
            if let Some(reason) = status {
                rejected.borrow_mut().push(format!("{refname}: {reason}"));
            }
            Ok(())
        });
        let mut opts = git2::PushOptions::new();
        opts.remote_callbacks(callbacks);
        let refs: Vec<&str> = refspecs.iter().map(String::as_str).collect();
        remote.push(&refs, Some(&mut opts))?;
    }
    let rejected = rejected.into_inner();
    if rejected.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(t("git_error_push_rejected").replace("{}", &rejected.join("; ")))
    }
}

/// Pushes local branch commits to the specified remote.
/// If `set_upstream` is true, configures upstream tracking for the branch.
pub fn push(
    repo: &git2::Repository,
    remote_name: &str,
    branch_name: &str,
    set_upstream: bool,
    hooks: NetHooks<'_>,
) -> anyhow::Result<()> {
    let mut remote = repo.find_remote(remote_name)?;
    let local_ref = format!("refs/heads/{branch_name}");
    let destination = if set_upstream {
        local_ref.clone()
    } else {
        upstream_merge_ref(repo, &local_ref, remote_name).unwrap_or_else(|| local_ref.clone())
    };
    push_refspecs(&mut remote, &[format!("{local_ref}:{destination}")], hooks)?;

    if set_upstream {
        let mut local_branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
        local_branch.set_upstream(Some(&format!("{remote_name}/{branch_name}")))?;
    }

    Ok(())
}

/// The branch `local_ref` tracks on `remote_name` (`branch.<name>.merge`),
/// which may be named differently from the local branch.
fn upstream_merge_ref(
    repo: &git2::Repository,
    local_ref: &str,
    remote_name: &str,
) -> Option<String> {
    use crate::git::remote::config_value as value;
    (value(repo.branch_upstream_remote(local_ref))? == remote_name)
        .then(|| value(repo.branch_upstream_merge(local_ref)))
        .flatten()
}

/// Deletes a remote branch on the specified remote.
pub fn delete_remote_branch(
    repo: &git2::Repository,
    remote_name: &str,
    branch_name: &str,
    hooks: NetHooks<'_>,
) -> anyhow::Result<()> {
    let clean_branch = branch_name
        .strip_prefix(&format!("{remote_name}/"))
        .unwrap_or(branch_name);

    let mut remote = repo.find_remote(remote_name)?;
    let refspec = format!(":refs/heads/{clean_branch}");
    push_refspecs(&mut remote, &[refspec], hooks)?;

    // Also remove local tracking reference if it exists
    let refname = format!("refs/remotes/{remote_name}/{clean_branch}");
    if let Ok(mut r) = repo.find_reference(&refname) {
        let _ = r.delete();
    }

    Ok(())
}
