use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub name: String,
    pub url: Option<String>,
    pub push_url: Option<String>,
}

/// Helper to configure SSH, credential helper, and basic authentication callbacks.
pub fn create_callbacks() -> git2::RemoteCallbacks<'static> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|url, username_from_url, allowed_types| {
        let username = username_from_url.unwrap_or("git");

        // 1. Try SSH Key authentication
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            // Try SSH agent first
            if let Ok(cred) = git2::Cred::ssh_key_from_agent(username) {
                return Ok(cred);
            }
            // Check standard SSH keys in ~/.ssh
            if let Some(proj_dir) = directories::BaseDirs::new() {
                let home_ssh: PathBuf = proj_dir.home_dir().join(".ssh");
                for key_name in &["id_ed25519", "id_ecdsa", "id_rsa"] {
                    let key_path = home_ssh.join(key_name);
                    if key_path.exists()
                        && let Ok(cred) = git2::Cred::ssh_key(username, None, &key_path, None)
                    {
                        return Ok(cred);
                    }
                }
            }
        }

        // 2. Try git credential helper (e.g. Git Credential Manager on Windows/macOS/Linux)
        if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT)
            && let Ok(config) = git2::Config::open_default()
            && let Ok(cred) = git2::Cred::credential_helper(&config, url, username_from_url)
        {
            return Ok(cred);
        }

        // 3. Try default credentials
        if allowed_types.contains(git2::CredentialType::DEFAULT)
            && let Ok(cred) = git2::Cred::default()
        {
            return Ok(cred);
        }

        Err(git2::Error::from_str(
            "Authentication failed or no credentials found",
        ))
    });
    callbacks
}

/// Dynamically resolves the remote to use for operations:
/// 1. If branch_name is given, checks if it has an upstream configured.
/// 2. If no upstream, checks if "origin" remote exists.
/// 3. If "origin" does not exist, returns the first available remote.
pub fn resolve_remote_name(
    repo: &git2::Repository,
    branch_name: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(b_name) = branch_name
        && let Ok(local_branch) = repo.find_branch(b_name, git2::BranchType::Local)
        && let Ok(upstream) = local_branch.upstream()
        && let Ok(Some(upstream_name)) = upstream.name()
        && let Some((remote, _)) = upstream_name.split_once('/')
    {
        return Ok(remote.to_string());
    }

    // Check if "origin" exists
    if repo.find_remote("origin").is_ok() {
        return Ok("origin".to_string());
    }

    // Otherwise use first available remote
    let remotes = repo.remotes()?;
    for name in &remotes {
        if let Ok(Some(n)) = name {
            return Ok(n.to_string());
        }
    }

    anyhow::bail!("No git remote configured")
}

/// Lists all configured remotes with their URLs.
pub fn list_remotes(repo: &git2::Repository) -> anyhow::Result<Vec<RemoteInfo>> {
    let remotes = repo.remotes()?;
    let mut result = Vec::new();
    for name in &remotes {
        if let Ok(Some(n)) = name
            && let Ok(remote) = repo.find_remote(n)
        {
            result.push(RemoteInfo {
                name: n.to_string(),
                url: remote.url().ok().map(|s| s.to_string()),
                push_url: remote.pushurl().ok().flatten().map(|s| s.to_string()),
            });
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

/// Adds a new remote to the repository.
pub fn add_remote(repo: &git2::Repository, name: &str, url: &str) -> anyhow::Result<()> {
    repo.remote(name, url)?;
    Ok(())
}

/// Deletes a remote by name.
pub fn delete_remote(repo: &git2::Repository, name: &str) -> anyhow::Result<()> {
    repo.remote_delete(name)?;
    Ok(())
}

/// Deletes a remote branch on the specified remote.
pub fn delete_remote_branch(
    repo: &git2::Repository,
    remote_name: &str,
    branch_name: &str,
) -> anyhow::Result<()> {
    let clean_branch =
        if let Some(stripped) = branch_name.strip_prefix(&format!("{}/", remote_name)) {
            stripped
        } else {
            branch_name
        };

    let mut remote = repo.find_remote(remote_name)?;
    let mut opts = git2::PushOptions::new();
    opts.remote_callbacks(create_callbacks());

    let refspec = format!(":refs/heads/{}", clean_branch);
    remote.push(&[refspec.as_str()], Some(&mut opts))?;

    // Also remove local tracking reference if it exists
    let refname = format!("refs/remotes/{}/{}", remote_name, clean_branch);
    if let Ok(mut r) = repo.find_reference(&refname) {
        let _ = r.delete();
    }

    Ok(())
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

        let sig = match repo.signature() {
            Ok(s) => s,
            Err(_) => git2::Signature::now("Pairee User", "pairee@localhost")
                .map_err(|e| anyhow::anyhow!("Failed to build fallback git signature: {}", e))?,
        };
        let message = format!(
            "Merge branch '{}/{}' into {}",
            remote_name, branch_name, branch_name
        );

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
/// If `set_upstream` is true, configures upstream tracking for the branch.
pub fn push(
    repo: &git2::Repository,
    remote_name: &str,
    branch_name: &str,
    set_upstream: bool,
) -> anyhow::Result<()> {
    let mut remote = repo.find_remote(remote_name)?;
    let mut opts = git2::PushOptions::new();
    opts.remote_callbacks(create_callbacks());

    let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
    remote.push(&[refspec.as_str()], Some(&mut opts))?;

    if set_upstream
        && let Ok(mut local_branch) = repo.find_branch(branch_name, git2::BranchType::Local)
    {
        let upstream_ref = format!("{}/{}", remote_name, branch_name);
        let _ = local_branch.set_upstream(Some(&upstream_ref));
    }

    Ok(())
}
