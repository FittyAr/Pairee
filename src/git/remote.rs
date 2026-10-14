use crate::credential_prompt::{self, Ask};
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

/// What a network operation reports to and asks of whoever started it:
/// progress (and cancellation) and the passphrase prompt. Each job passes
/// the asker of the state that started it, like remote connections do.
#[derive(Clone, Copy)]
pub struct NetHooks<'a> {
    pub progress: ProgressObserver<'a>,
    pub ask: Ask<'a>,
}

impl NetHooks<'_> {
    /// `false` once the caller cancelled the operation (asked through the
    /// progress observer, without any progress to report).
    pub fn still_wanted(&self) -> bool {
        self.progress
            .is_none_or(|observe| observe(TransferStats::default()))
    }
}

impl Default for NetHooks<'_> {
    /// No progress, and nobody to ask (every prompt is cancelled).
    fn default() -> Self {
        Self {
            progress: None,
            ask: &credential_prompt::nobody,
        }
    }
}

/// Remote callbacks with bounded credential negotiation (see
/// [`crate::git::credentials`]) plus optional progress reporting / cancellation.
pub fn create_callbacks(hooks: NetHooks<'_>) -> git2::RemoteCallbacks<'_> {
    let mut callbacks = git2::RemoteCallbacks::new();
    if let Some(obs) = hooks.progress {
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
            if hooks.still_wanted() {
                Ok(())
            } else {
                Err(git2::Error::from_str(&crate::config::localization::t(
                    "git_operation_cancelled",
                )))
            }
        });
    }
    let mut credentials = crate::git::credentials::CredentialState::with_default_keys(hooks.ask);
    callbacks.credentials(move |url, username_from_url, allowed_types| {
        credentials.next_credential(url, username_from_url, allowed_types)
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

    anyhow::bail!(crate::config::localization::t("git_error_no_remote"))
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
    hooks: NetHooks<'_>,
) -> anyhow::Result<()> {
    let mut remote = repo.find_remote(remote_name)?;
    let mut opts = git2::FetchOptions::new();
    opts.remote_callbacks(create_callbacks(hooks));
    remote.fetch(&[] as &[&str], Some(&mut opts), None)?;
    Ok(())
}

/// A branch setting libgit2 returns as a buffer (`branch_upstream_*`), or
/// `None` when it is unset or not UTF-8.
pub fn config_value(buf: Result<git2::Buf, git2::Error>) -> Option<String> {
    buf.ok().and_then(|b| b.as_str().ok().map(str::to_string))
}
