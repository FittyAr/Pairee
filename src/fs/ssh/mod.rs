//! SSH client and shared handle for remote operations.
//!
//! Every method here performs network I/O: call them only from background
//! jobs / transfer workers, never from the UI thread. Display data (host,
//! user) lives outside the mutex so the UI can show it without locking.

mod connection;
mod du_source;
mod remote_fs;
mod sftp_ops;
#[cfg(test)]
mod tests;

use crate::config::localization::t;
use crate::fs::entry::FileEntry;
use crate::lock::LockExt;
use anyhow::Result;
use ssh2::{Session, Sftp};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

/// Default timeout for blocking libssh2 calls (see `ssh_timeout_secs`).
pub const DEFAULT_SSH_TIMEOUT_SECS: u64 = 30;

pub struct SshClient {
    /// Kept alive for the lifetime of the SFTP channel.
    pub session: Session,
    pub sftp: Sftp,
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

    pub fn info(&self) -> &SshInfo {
        &self.info
    }

    /// Exclusive access to the session (poison-tolerant).
    pub fn lock(&self) -> MutexGuard<'_, SshClient> {
        self.inner.lock_safe()
    }

    /// Connects, verifies the host key and authenticates. Blocking calls on
    /// the session time out after `timeout` (0 disables the limit).
    pub fn connect(
        host: &str,
        port: u16,
        username: &str,
        password: Option<&str>,
        key_path: Option<&str>,
        timeout: Duration,
    ) -> Result<Self> {
        let session = connection::open_session(host, port, username, password, key_path, timeout)?;
        let sftp = session
            .sftp()
            .map_err(|e| anyhow::anyhow!("{}: {}", t("error_ssh_init_sftp"), e))?;

        Ok(Self {
            inner: Arc::new(Mutex::new(SshClient { session, sftp })),
            info: Arc::new(SshInfo {
                host: host.to_string(),
                port,
                username: username.to_string(),
            }),
        })
    }

    pub fn read_directory(
        &self,
        path: &Path,
        opts: &crate::fs::list::ListOptions,
    ) -> Result<Vec<FileEntry>> {
        sftp_ops::read_directory(&self.lock().sftp, path, opts)
    }

    pub fn create_dir(&self, path: &Path) -> Result<()> {
        self.lock().sftp.mkdir(path, 0o755)?;
        Ok(())
    }

    pub fn delete_recursive(&self, path: &Path) -> Result<()> {
        remote_fs::delete_recursive(&self.lock().sftp, path)
    }

    pub fn walk_dir(&self, root: &Path) -> Result<Vec<(PathBuf, bool, u64)>> {
        sftp_ops::walk_dir(&self.lock().sftp, root)
    }

    pub fn rename_move(&self, src: &Path, dst: &Path) -> Result<()> {
        self.lock().sftp.rename(src, dst, None)?;
        Ok(())
    }

    /// `true` when `path` exists on the server and is a directory.
    pub fn is_dir(&self, path: &Path) -> bool {
        self.lock()
            .sftp
            .stat(path)
            .map(|s| s.is_dir())
            .unwrap_or(false)
    }

    /// Size of a remote file (0 when unknown).
    pub fn file_size(&self, path: &Path) -> u64 {
        self.lock()
            .sftp
            .stat(path)
            .ok()
            .and_then(|s| s.size)
            .unwrap_or(0)
    }
}

/// `is_dir` on either side of a transfer: remote when `conn` is set.
pub fn is_dir_on(path: &Path, conn: &Option<SharedSshClient>) -> bool {
    match conn {
        Some(client) => client.is_dir(path),
        None => path.is_dir(),
    }
}
