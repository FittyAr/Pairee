use crate::config::localization::t;
use anyhow::{Result, anyhow};

/// A localized error `key` (with `{}` replaced by `detail`).
fn failed(key: &str, detail: impl std::fmt::Display) -> anyhow::Error {
    anyhow!(t(key).replace("{}", &detail.to_string()))
}

/// Rebase the current branch onto `onto_branch_name`.
pub fn rebase_branch(repo: &git2::Repository, onto_branch_name: &str) -> Result<()> {
    // Find the target branch reference and annotated commit
    let onto_branch = repo
        .find_branch(onto_branch_name, git2::BranchType::Local)
        .or_else(|_| repo.find_branch(onto_branch_name, git2::BranchType::Remote))
        .map_err(|_| failed("git_error_branch_not_found", onto_branch_name))?;

    crate::git::outcome::ensure_no_pending(repo)?;
    let onto_ref = onto_branch.get();
    let onto_annotated = repo.reference_to_annotated_commit(onto_ref)?;

    let mut rebase_opts = git2::RebaseOptions::new();
    let mut rebase = repo
        .rebase(None, Some(&onto_annotated), None, Some(&mut rebase_opts))
        .map_err(|e| failed("git_error_rebase_start", e))?;

    let sig = crate::git::signature::default_signature(repo)?;

    while let Some(op_res) = rebase.next() {
        if let Err(e) = op_res {
            let _ = rebase.abort();
            return Err(failed("git_error_rebase_step", e));
        }

        let index = repo.index()?;
        if index.has_conflicts() {
            let _ = rebase.abort();
            return Err(anyhow!(t("git_error_rebase_conflicts")));
        }

        match rebase.commit(None, &sig, None) {
            Ok(_) => {}
            // The step's changes are already upstream (or it is empty):
            // skip it like `git rebase` does instead of aborting.
            Err(e) if e.code() == git2::ErrorCode::Applied => {}
            Err(e) => {
                let _ = rebase.abort();
                return Err(failed("git_error_rebase_step", e));
            }
        }
    }

    rebase
        .finish(None)
        .map_err(|e| failed("git_error_rebase_finish", e))?;
    Ok(())
}
