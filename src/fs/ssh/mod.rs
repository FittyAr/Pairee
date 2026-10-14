//! SSH client and shared handle for remote operations.
//!
//! Every method here performs network I/O: call them only from background
//! jobs / transfer workers, never from the UI thread. Display data (host,
//! user) lives outside the mutex so the UI can show it without locking.

mod connection;
#[cfg(all(test, pairee_sftp_test))]
pub(crate) mod server_tests;
mod sftp_ops;
#[cfg(test)]
mod tests;
mod vfs;

use crate::config::localization::t;
use crate::credential_prompt::Ask;
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
    pub fn connect(login: &SshLogin<'_>) -> Result<Self> {
        let session = connection::open_session(login)?;
        let sftp = session
            .sftp()
            .map_err(|e| anyhow::anyhow!("{}: {}", t("error_ssh_init_sftp"), e))?;

        Ok(Self {
            inner: Arc::new(Mutex::new(SshClient { session, sftp })),
            info: Arc::new(SshInfo {
                host: login.host.to_string(),
                port: login.port,
                username: login.username.to_string(),
            }),
        })
    }
}
