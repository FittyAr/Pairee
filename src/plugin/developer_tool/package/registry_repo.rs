use super::super::find_pairee_repo;

pub fn fetch_or_clone_registry(temp_dir: &std::path::Path) -> anyhow::Result<git2::Repository> {
    if temp_dir.join(".git").exists() {
        if let Ok(repo) = git2::Repository::open(temp_dir) {
            let fetched = {
                if let Ok(mut remote) = repo.find_remote("origin") {
                    let mut fetch_options = git2::FetchOptions::new();
                    remote
                        .fetch(
                            &["+refs/heads/plugin-registry:refs/remotes/origin/plugin-registry"],
                            Some(&mut fetch_options),
                            None,
                        )
                        .is_ok()
                } else {
                    false
                }
            };
            let mut reset_ok = false;
            let mut commit_oid = None;
            if fetched
                && let Ok(fetch_head) = repo.find_reference("refs/remotes/origin/plugin-registry")
                && let Ok(commit) = fetch_head.peel_to_commit()
            {
                commit_oid = Some(commit.id());
            }
            if let Some(oid) = commit_oid
                && let Ok(commit) = repo.find_commit(oid)
            {
                let mut checkout_builder = git2::build::CheckoutBuilder::new();
                checkout_builder.force();
                let _ = repo.checkout_tree(commit.as_object(), Some(&mut checkout_builder));
                let _ = repo.set_head("refs/heads/plugin-registry");
                if repo
                    .reset(commit.as_object(), git2::ResetType::Hard, None)
                    .is_ok()
                {
                    reset_ok = true;
                }
            }
            if reset_ok {
                return Ok(repo);
            }
        }
        // If anything fails, clean up and clone
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    std::fs::create_dir_all(temp_dir)?;

    let url = if let Some(local_path) = find_pairee_repo() {
        log::debug!(
            "plugin-registry: Using local repository clone for registry: {:?}",
            local_path
        );
        local_path.to_string_lossy().into_owned()
    } else {
        log::debug!(
            "plugin-registry: Local repo not found. Cloning registry from remote GitHub..."
        );
        "https://github.com/FittyAr/Pairee.git".to_string()
    };

    let mut builder = git2::build::RepoBuilder::new();
    builder.branch("plugin-registry");
    let repo = builder.clone(&url, temp_dir)?;
    Ok(repo)
}
