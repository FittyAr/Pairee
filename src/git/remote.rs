use std::path::PathBuf;

pub use crate::git::pull::pull;
pub use crate::git::push::{delete_remote_branch, push};

#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub name: String,
    pub url: Option<String>,
    pub push_url: Option<String>,
}

/// Network transfer statistics reported during fetch, push and clone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransferStats {
    /// Objects received (fetch/clone) or sent (push) so far.
    pub done: usize,
    /// Total objects to transfer (0 while unknown).
    pub total: usize,
    /// Bytes transferred so far.
    pub bytes: usize,
}

/// Progress observer; returning `false` aborts the transfer (cancellation).
pub type ProgressObserver<'a> = Option<&'a (dyn Fn(TransferStats) -> bool + 'a)>;

/// Helper to configure SSH, credential helper, and basic authentication
/// callbacks, plus optional progress reporting / cancellation.
pub fn create_callbacks(observer: ProgressObserver<'_>) -> git2::RemoteCallbacks<'_> {
    let mut callbacks = git2::RemoteCallbacks::new();
    if let Some(obs) = observer {
        callbacks.transfer_progress(move |p| {
            obs(TransferStats {
                done: p.received_objects(),
                total: p.total_objects(),
                bytes: p.received_bytes(),
            })
        });
        callbacks.push_transfer_progress(move |done, total, bytes| {
            // Push progress cannot abort; cancellation is honoured at
            // negotiation time below.
            let _ = obs(TransferStats { done, total, bytes });
        });
        callbacks.push_negotiation(move |_| {
            if obs(TransferStats::default()) {
                Ok(())
            } else {
                Err(git2::Error::from_str(&crate::config::localization::t(
                    "git_operation_cancelled",
                )))
            }
        });
    }
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

/// Fetches objects and refs from the specified remote.
pub fn fetch(
    repo: &git2::Repository,
    remote_name: &str,
    observer: ProgressObserver<'_>,
) -> anyhow::Result<()> {
    let mut remote = repo.find_remote(remote_name)?;
    let mut opts = git2::FetchOptions::new();
    opts.remote_callbacks(create_callbacks(observer));
    remote.fetch(&[] as &[&str], Some(&mut opts), None)?;
    Ok(())
}
