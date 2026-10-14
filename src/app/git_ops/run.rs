//! Blocking execution of a [`GitNetOp`] (runs on the job thread).

use super::{GitNetOp, GitOpResult};
use crate::app::jobs::JobContext;
use crate::credential_prompt::Asker;
use crate::git::outcome::Outcome;
use crate::git::remote::{self, NetHooks, TransferStats};
use std::path::Path;

/// Runs `op`, reporting progress to `ctx` and asking passphrases through
/// `asker` (the prompts of the state that started it).
pub fn run(op: &GitNetOp, ctx: &JobContext<TransferStats>, asker: &Asker) -> GitOpResult {
    let observer = |stats: TransferStats| {
        ctx.report(stats);
        !ctx.is_cancelled()
    };
    let ask = |request| asker.ask(request);
    let hooks = NetHooks {
        progress: Some(&observer),
        ask: &ask,
    };
    let res = execute(op, hooks);
    if ctx.is_cancelled() {
        return Err(None);
    }
    res.map_err(|e| Some(e.to_string()))
}

/// Runs `op`; `Ok` carries the files a pull left conflicted (usually none).
fn execute(op: &GitNetOp, hooks: NetHooks<'_>) -> anyhow::Result<Vec<String>> {
    let done = |result: anyhow::Result<()>| result.map(|()| Vec::new());
    match op {
        GitNetOp::Fetch { repo_path } => {
            let repo = open(repo_path)?;
            let remote_name = remote::resolve_remote_name(&repo, None)?;
            done(remote::fetch(&repo, &remote_name, hooks))
        }
        GitNetOp::Pull { repo_path, branch } => {
            let repo = open(repo_path)?;
            let remote_name = remote::resolve_remote_name(&repo, Some(branch))?;
            match remote::pull(&repo, &remote_name, branch, hooks)? {
                Outcome::Conflicts(paths) => Ok(paths),
                _ => Ok(Vec::new()),
            }
        }
        GitNetOp::Push {
            repo_path,
            branch,
            set_upstream,
        } => {
            let repo = open(repo_path)?;
            let remote_name = remote::resolve_remote_name(&repo, Some(branch))?;
            done(remote::push(
                &repo,
                &remote_name,
                branch,
                *set_upstream,
                hooks,
            ))
        }
        GitNetOp::PushTags {
            repo_path,
            remote: remote_name,
        } => done(crate::git::tags::push_tags(
            &open(repo_path)?,
            remote_name,
            hooks,
        )),
        GitNetOp::DeleteRemoteBranch {
            repo_path,
            remote: remote_name,
            branch,
        } => done(remote::delete_remote_branch(
            &open(repo_path)?,
            remote_name,
            branch,
            hooks,
        )),
        GitNetOp::Clone { url, target } => {
            done(crate::git::repo::clone_repo(url, target, hooks).map(|_| ()))
        }
    }
}

fn open(path: &Path) -> anyhow::Result<git2::Repository> {
    crate::git::repo::find_repo(path)
        .ok_or_else(|| anyhow::anyhow!(crate::config::localization::t("git_error_no_repo")))
}
