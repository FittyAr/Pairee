//! Credential negotiation for fetch, pull, push and clone.
//!
//! libgit2 calls the credential callback again every time an attempt is
//! rejected, so each method is offered at most once and the total number of
//! attempts is capped: a wrong key can never turn into an endless loop.

use crate::config::localization::t;
use std::path::PathBuf;

/// Hard cap on credential callback invocations for one transfer.
const MAX_CREDENTIAL_ATTEMPTS: usize = 8;
/// Username used for SSH when the URL does not carry one.
const DEFAULT_SSH_USER: &str = "git";
/// Private keys probed in `~/.ssh`, in order of preference.
const DEFAULT_KEY_NAMES: [&str; 3] = ["id_ed25519", "id_ecdsa", "id_rsa"];

/// What has already been offered to the remote during one transfer.
pub struct CredentialState {
    attempts: usize,
    agent_tried: bool,
    key_paths: Vec<PathBuf>,
    next_key: usize,
    helper_tried: bool,
    default_tried: bool,
}

impl CredentialState {
    /// A fresh negotiation that will probe `key_paths` after the ssh-agent.
    pub fn new(key_paths: Vec<PathBuf>) -> Self {
        Self {
            attempts: 0,
            agent_tried: false,
            key_paths,
            next_key: 0,
            helper_tried: false,
            default_tried: false,
        }
    }

    /// A negotiation using the user's default keys in `~/.ssh`.
    pub fn with_default_keys() -> Self {
        Self::new(default_key_paths())
    }

    /// Returns the next credential to offer, or an error once every method
    /// (ssh-agent, default keys, credential helper, default credentials) has
    /// been tried or the attempt cap is reached.
    pub fn next_credential(
        &mut self,
        url: &str,
        username_from_url: Option<&str>,
        allowed: git2::CredentialType,
    ) -> Result<git2::Cred, git2::Error> {
        self.attempts += 1;
        if self.attempts > MAX_CREDENTIAL_ATTEMPTS {
            return Err(failure(allowed));
        }
        let username = username_from_url.unwrap_or(DEFAULT_SSH_USER);
        if allowed.contains(git2::CredentialType::USERNAME) {
            return git2::Cred::username(username);
        }
        if allowed.contains(git2::CredentialType::SSH_KEY)
            && let Some(cred) = self.next_ssh_key(username)
        {
            return Ok(cred);
        }
        if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) && !self.helper_tried {
            self.helper_tried = true;
            if let Ok(config) = git2::Config::open_default()
                && let Ok(cred) = git2::Cred::credential_helper(&config, url, username_from_url)
            {
                return Ok(cred);
            }
        }
        if allowed.contains(git2::CredentialType::DEFAULT) && !self.default_tried {
            self.default_tried = true;
            if let Ok(cred) = git2::Cred::default() {
                return Ok(cred);
            }
        }
        Err(failure(allowed))
    }

    /// ssh-agent first, then each existing default key, each exactly once.
    fn next_ssh_key(&mut self, username: &str) -> Option<git2::Cred> {
        if !self.agent_tried {
            self.agent_tried = true;
            if let Ok(cred) = git2::Cred::ssh_key_from_agent(username) {
                return Some(cred);
            }
        }
        while let Some(path) = self.key_paths.get(self.next_key) {
            self.next_key += 1;
            if path.exists()
                && let Ok(cred) = git2::Cred::ssh_key(username, None, path, None)
            {
                return Some(cred);
            }
        }
        None
    }
}

/// `~/.ssh/<name>` for every default key name.
fn default_key_paths() -> Vec<PathBuf> {
    directories::BaseDirs::new()
        .map(|dirs| {
            let ssh_dir = dirs.home_dir().join(".ssh");
            DEFAULT_KEY_NAMES.iter().map(|n| ssh_dir.join(n)).collect()
        })
        .unwrap_or_default()
}

/// Localized authentication error; SSH failures explain the passphrase case
/// (Pairee cannot prompt for one, so such keys must be in ssh-agent).
fn failure(allowed: git2::CredentialType) -> git2::Error {
    let key = if allowed.contains(git2::CredentialType::SSH_KEY) {
        "git_error_ssh_auth_failed"
    } else {
        "git_error_auth_failed"
    };
    git2::Error::from_str(&t(key))
}
