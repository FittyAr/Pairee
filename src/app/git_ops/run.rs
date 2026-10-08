//! Blocking execution of a [`GitNetOp`] (runs on the job thread).

use super::{GitNetOp, GitOpResult};
use crate::app::jobs::JobContext;
use crate::git::remote::{self, TransferStats};
use std::path::Path;

pub fn run(op: &GitNetOp, ctx: &JobContext<TransferStats>) -> GitOpResult {
    let observer = |stats: TransferStats| {
        ctx.report(stats);
        !ctx.is_cancelled()
    };
    let res = execute(op, Some(&observer));
    if ctx.is_cancelled() {
        return Err(None);
    }
    res.map_err(|e| Some(e.to_string()))
}

fn execute(op: &GitNetOp, observer: remote::ProgressObserver<'_>) -> anyhow::Result<()> {
    match op {
        GitNetOp::Fetch { repo_path } => {
            let repo = open(repo_path)?;
            let remote_name = remote::resolve_remote_name(&repo, None)?;
            remote::fetch(&repo, &remote_name, observer)
        }
        GitNetOp::Pull { repo_path, branch } => {
            let repo = open(repo_path)?;
            let remote_name = remote::resolve_remote_name(&repo, Some(branch))?;
            remote::pull(&repo, &remote_name, branch, observer)
        }
        GitNetOp::Push {
            repo_path,
            branch,
            set_upstream,
        } => {
            let repo = open(repo_path)?;
            let remote_name = remote::resolve_remote_name(&repo, Some(branch))?;
            remote::push(&repo, &remote_name, branch, *set_upstream, observer)
        }
        GitNetOp::PushTags {
            repo_path,
            remote: remote_name,
        } => crate::git::tags::push_tags(&open(repo_path)?, remote_name, observer),
        GitNetOp::DeleteRemoteBranch {
            repo_path,
            remote: remote_name,
            branch,
        } => remote::delete_remote_branch(&open(repo_path)?, remote_name, branch, observer),
        GitNetOp::Clone { url, target } => {
            crate::git::repo::clone_repo(url, target, observer).map(|_| ())
        }
    }
}

fn open(path: &Path) -> anyhow::Result<git2::Repository> {
    crate::git::repo::find_repo(path)
        .ok_or_else(|| anyhow::anyhow!(crate::config::localization::t("git_error_no_repo")))
}
