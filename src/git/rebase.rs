use anyhow::{Context, Result, anyhow};

/// Rebase the current branch onto `onto_branch_name`.
pub fn rebase_branch(repo: &git2::Repository, onto_branch_name: &str) -> Result<()> {
    // Find the target branch reference and annotated commit
    let onto_branch = repo
        .find_branch(onto_branch_name, git2::BranchType::Local)
        .or_else(|_| repo.find_branch(onto_branch_name, git2::BranchType::Remote))
        .with_context(|| format!("Branch '{}' not found", onto_branch_name))?;

    let onto_ref = onto_branch.get();
    let onto_annotated = repo.reference_to_annotated_commit(onto_ref)?;

    let mut rebase_opts = git2::RebaseOptions::new();
    let mut rebase = repo
        .rebase(None, Some(&onto_annotated), None, Some(&mut rebase_opts))
        .context("Failed to initialize rebase")?;

    let sig = crate::git::signature::default_signature(repo)?;

    while let Some(op_res) = rebase.next() {
        let _op = match op_res {
            Ok(op) => op,
            Err(e) => {
                let _ = rebase.abort();
                return Err(anyhow!("Rebase step failed: {}", e));
            }
        };

        let index = repo.index()?;
        if index.has_conflicts() {
            let _ = rebase.abort();
            return Err(anyhow!(
                "Conflicts detected during rebase; operation aborted"
            ));
        }

        match rebase.commit(None, &sig, None) {
            Ok(_) => {}
            // The step's changes are already upstream (or it is empty):
            // skip it like `git rebase` does instead of aborting.
            Err(e) if e.code() == git2::ErrorCode::Applied => {}
            Err(e) => {
                let _ = rebase.abort();
                return Err(anyhow!("Failed to commit rebase step: {}", e));
            }
        }
    }

    rebase.finish(None).context("Failed to finish rebase")?;
    Ok(())
}
