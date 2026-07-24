
/// Helper to configure SSH and basic authentication callbacks.
fn create_callbacks() -> git2::RemoteCallbacks<'static> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, allowed_types| {
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            let username = username_from_url.unwrap_or("git");
            if let Ok(cred) = git2::Cred::ssh_key_from_agent(username) {
                return Ok(cred);
            }
            if let Some(proj_dir) = directories::BaseDirs::new() {
                let mut home = proj_dir.home_dir().to_path_buf();
                home.push(".ssh");
                let id_rsa = home.join("id_rsa");
                if id_rsa.exists() {
                    if let Ok(cred) = git2::Cred::ssh_key(username, None, &id_rsa, None) {
                        return Ok(cred);
                    }
                }
            }
        }
        Err(git2::Error::from_str("Authentication failed or no credentials found"))
    });
    callbacks
}

/// Fetches objects and refs from the specified remote.
pub fn fetch(repo: &git2::Repository, remote_name: &str) -> anyhow::Result<()> {
    let mut remote = repo.find_remote(remote_name)?;
    let mut opts = git2::FetchOptions::new();
    opts.remote_callbacks(create_callbacks());
    remote.fetch(&[] as &[&str], Some(&mut opts), None)?;
    Ok(())
}

/// Pulls changes from the specified remote and branch into the current branch.
pub fn pull(repo: &git2::Repository, remote_name: &str, branch_name: &str) -> anyhow::Result<()> {
    // 1. Fetch first
    fetch(repo, remote_name)?;

    // 2. Resolve remote reference
    let remote_ref_name = format!("refs/remotes/{}/{}", remote_name, branch_name);
    let remote_ref = repo.find_reference(&remote_ref_name)?;
    let annotated_commit = repo.reference_to_annotated_commit(&remote_ref)?;

    // 3. Analyze merge
    let (analysis, _) = repo.merge_analysis(&[&annotated_commit])?;

    if analysis.is_fast_forward() {
        let refname = format!("refs/heads/{}", branch_name);
        let mut reference = repo.find_reference(&refname)?;
        reference.set_target(annotated_commit.id(), "pull: Fast-forward")?;
        repo.set_head(&refname)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
    } else if analysis.is_normal() {
        let local_commit = repo.head()?.peel_to_commit()?;
        let remote_commit = repo.find_commit(annotated_commit.id())?;

        let mut index = repo.merge_commits(&local_commit, &remote_commit, None)?;
        if index.has_conflicts() {
            anyhow::bail!("Merge conflicts detected. Please resolve conflicts manually.");
        }

        let tree_id = index.write_tree_to(repo)?;
        let tree = repo.find_tree(tree_id)?;

        let sig = repo.signature().unwrap_or_else(|_| {
            git2::Signature::now("Pairee User", "pairee@localhost").unwrap()
        });
        let message = format!("Merge branch '{}/{}' into {}", remote_name, branch_name, branch_name);

        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &message,
            &tree,
            &[&local_commit, &remote_commit],
        )?;

        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
    } else if analysis.is_up_to_date() {
        // Already up to date, nothing to do
    } else {
        anyhow::bail!("Unsupported merge analysis result: {:?}", analysis);
    }

    Ok(())
}

/// Pushes local branch commits to the specified remote.
pub fn push(repo: &git2::Repository, remote_name: &str, branch_name: &str) -> anyhow::Result<()> {
    let mut remote = repo.find_remote(remote_name)?;
    let mut opts = git2::PushOptions::new();
    opts.remote_callbacks(create_callbacks());

    let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
    remote.push(&[refspec.as_str()], Some(&mut opts))?;
    Ok(())
}
