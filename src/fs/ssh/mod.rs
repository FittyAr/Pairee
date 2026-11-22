//! SSH client and shared handle for remote operations.
//!
//! Every method here performs network I/O: call them only from background
//! jobs / transfer workers, never from the UI thread. Display data (host,
//! user) lives outside the mutex so the UI can show it without locking.

mod connection;
mod known_hosts;
mod pty;
#[cfg(all(test, pairee_sftp_test))]
pub(crate) mod server_tests;
mod sftp_ops;
#[cfg(test)]
mod tests;
mod vfs;

use crate::config::localization::t;
use crate::credential_prompt::{Ask, Secret};
use crate::lock::LockExt;
use anyhow::Result;
use ssh2::{Session, Sftp};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

/// Default timeout for blocking libssh2 calls (see `ssh_timeout_secs`).
pub const DEFAULT_SSH_TIMEOUT_SECS: u64 = 30;

pub struct SshClient {
    /// Kept alive for the lifetime of the SFTP channel.
    pub session: Session,
    pub sftp: Sftp,
}

/// Where to connect and how to log in.
pub struct SshLogin<'a> {
    pub host: &'a str,
    pub port: u16,
    pub username: &'a str,
    /// The password, also the passphrase of an encrypted `key_path`.
    pub password: Option<&'a str>,
    pub key_path: Option<&'a str>,
    pub timeout: Duration,
    /// Asks the user for a missing password or key passphrase.
    pub ask: Ask<'a>,
}

/// Connection identity, readable without locking the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshInfo {
    pub host: String,
    pub port: u16,
    pub username: String,
}

#[derive(Clone)]
pub struct SharedSshClient {
    inner: Arc<Mutex<SshClient>>,
    info: Arc<SshInfo>,
    /// What it takes to open the session again after the server dropped it.
    login: Arc<SavedLogin>,
}

/// The parts of an [`SshLogin`] kept for [`SharedSshClient::open_session`].
struct SavedLogin {
    password: Option<zeroize::Zeroizing<String>>,
    key_path: Option<String>,
    timeout: Duration,
}

impl SavedLogin {
    /// Keeps the secret the user typed for `request` (the one that logged
    /// in): a password, or the passphrase of the key it names.
    fn remember(&mut self, request: &crate::credential_prompt::CredentialRequest, secret: Secret) {
        use crate::credential_prompt::SecretKind;
        match request.kind {
            SecretKind::Password => self.password = Some(secret),
            SecretKind::Passphrase => {
                self.key_path = Some(request.subject.clone());
                self.password = Some(secret);
            }
            SecretKind::Certificate => {}
        }
    }
}

impl std::fmt::Debug for SharedSshClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedSshClient")
            .field("host", &self.info.host)
            .field("port", &self.info.port)
            .field("username", &self.info.username)
            .finish()
    }
}

impl SharedSshClient {
    pub fn is_same_server(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// Identity of the connection (equal for clones of one client).
    pub fn id(&self) -> usize {
        Arc::as_ptr(&self.inner).addr()
    }

    pub fn info(&self) -> &SshInfo {
        &self.info
    }

    /// Exclusive access to the session (poison-tolerant).
    pub fn lock(&self) -> MutexGuard<'_, SshClient> {
        self.inner.lock_safe()
    }

    /// Connects, verifies the host key and authenticates. Blocking calls on
    /// the session time out after `login.timeout` (0 disables the limit).
    /// A password or key passphrase the user typed at a prompt is kept with
    /// the login, so the session can be opened again without asking (see
    /// [`Self::open_session`]).
    pub fn connect(login: &SshLogin<'_>) -> Result<Self> {
        let asked = std::sync::Mutex::new(None);
        let remember = |request: crate::credential_prompt::CredentialRequest| {
            let answer = (login.ask)(request.clone());
            if let Some(secret) = &answer {
                *asked.lock_safe() = Some((request, secret.clone()));
            }
            answer
        };
        let session = connection::open_session(&SshLogin {
            ask: &remember,
            ..*login
        })?;
        let sftp = session
            .sftp()
            .map_err(|e| anyhow::anyhow!("{}: {}", t("error_ssh_init_sftp"), e))?;
        let mut saved = SavedLogin {
            password: login
                .password
                .map(|p| zeroize::Zeroizing::new(p.to_string())),
            key_path: login.key_path.map(str::to_string),
            timeout: login.timeout,
        };
        if let Some((request, secret)) = asked.into_inner().unwrap_or_else(|e| e.into_inner()) {
            saved.remember(&request, secret);
        }

        Ok(Self {
            inner: Arc::new(Mutex::new(SshClient { session, sftp })),
            info: Arc::new(SshInfo {
                host: login.host.to_string(),
                port: login.port,
                username: login.username.to_string(),
            }),
            login: Arc::new(saved),
        })
    }

    /// A new, separate session to the same server with the saved login
    /// (terminals run on their own session so they never hold up the SFTP
    /// one). Never asks the user.
    pub(crate) fn open_session(&self) -> Result<Session> {
        let info = &self.info;
        let saved = &self.login;
        let ask = |_: crate::credential_prompt::CredentialRequest| -> Option<crate::credential_prompt::Secret> { None };
        connection::open_session(&SshLogin {
            host: &info.host,
            port: info.port,
            username: &info.username,
            password: saved.password.as_ref().map(|p| p.as_str()),
            key_path: saved.key_path.as_deref(),
            timeout: saved.timeout,
            ask: &ask,
        })
    }

    /// Opens the session again in place (every clone of the client sees the
    /// new one), with the saved login: a login that failed here reports the
    /// original error.
    fn reconnect(&self) -> Result<()> {
        let session = self.open_session()?;
        let sftp = session
            .sftp()
            .map_err(|e| anyhow::anyhow!("{}: {}", t("error_ssh_init_sftp"), e))?;
        *self.lock() = SshClient { session, sftp };
        Ok(())
    }
}
