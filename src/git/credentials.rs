//! Credential negotiation for fetch, pull, push and clone.
//!
//! libgit2 calls the credential callback again every time an attempt is
//! rejected, so each method is offered at most once and the total number of
//! attempts is capped: a wrong key can never turn into an endless loop.
//! An encrypted key is offered after asking the user for its passphrase
//! (see [`crate::credential_prompt`]); a cancelled prompt skips the key.

use crate::config::localization::t;
use crate::credential_prompt::{self, Ask, CredentialRequest};
use std::path::{Path, PathBuf};

/// Hard cap on credential callback invocations for one transfer.
const MAX_CREDENTIAL_ATTEMPTS: usize = 8;
/// Username used for SSH when the URL does not carry one.
const DEFAULT_SSH_USER: &str = "git";

/// Asks the user for a secret (`None`: cancelled or nobody to ask).
type Prompt<'a> = Box<dyn FnMut(CredentialRequest) -> Option<String> + 'a>;

/// What has already been offered to the remote during one transfer.
pub struct CredentialState<'a> {
    attempts: usize,
    agent_tried: bool,
    key_paths: Vec<PathBuf>,
    next_key: usize,
    helper_tried: bool,
    default_tried: bool,
    prompt: Prompt<'a>,
}

impl<'a> CredentialState<'a> {
    /// A fresh negotiation that will probe `key_paths` after the ssh-agent,
    /// asking passphrases through `ask` (the starting state's prompt).
    pub fn new(key_paths: Vec<PathBuf>, ask: Ask<'a>) -> Self {
        Self {
            attempts: 0,
            agent_tried: false,
            key_paths,
            next_key: 0,
            helper_tried: false,
            default_tried: false,
            prompt: Box::new(ask),
        }
    }

    /// The same negotiation, asking passphrases through `prompt` (tests).
    #[cfg(test)]
    pub fn with_prompt(
        mut self,
        prompt: impl FnMut(CredentialRequest) -> Option<String> + 'a,
    ) -> Self {
        self.prompt = Box::new(prompt);
        self
    }

    /// A negotiation using the user's default keys in `~/.ssh`.
    pub fn with_default_keys(ask: Ask<'a>) -> Self {
        Self::new(credential_prompt::key::default_key_paths(), ask)
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
        while let Some(path) = self.key_paths.get(self.next_key).cloned() {
            self.next_key += 1;
            if path.exists()
                && let Some(cred) = self.key_credential(username, &path)
            {
                return Some(cred);
            }
        }
        None
    }

    /// The credential of the key at `path`; an encrypted key needs its
    /// passphrase from the user (`None` when they cancel).
    fn key_credential(&mut self, username: &str, path: &Path) -> Option<git2::Cred> {
        let passphrase = if credential_prompt::key::is_encrypted(path) {
            Some((self.prompt)(CredentialRequest::passphrase(path))?)
        } else {
            None
        };
        git2::Cred::ssh_key(username, None, path, passphrase.as_deref()).ok()
    }
}

/// Localized authentication error (SSH failures name the methods tried).
fn failure(allowed: git2::CredentialType) -> git2::Error {
    let key = if allowed.contains(git2::CredentialType::SSH_KEY) {
        "git_error_ssh_auth_failed"
    } else {
        "git_error_auth_failed"
    };
    git2::Error::from_str(&t(key))
}
