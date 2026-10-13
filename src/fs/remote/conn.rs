//! A connected remote filesystem as panels, transfers, the journal and
//! auto-refresh see it: the adapter plus who it is connected to. Every
//! protocol (SFTP, FTP, FTPS, WebDAV, S3) looks the same from there.

use crate::config::settings::RemoteKind;
use crate::fs::ssh::SharedSshClient;
use crate::fs::vfs::Vfs;
use std::sync::Arc;

/// Who a connection reaches (for titles, sessions and matching saved
/// connections). Never holds a password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteInfo {
    pub kind: RemoteKind,
    /// Server name (S3: the bucket).
    pub host: String,
    /// `None`: the protocol's endpoint decides (S3).
    pub port: Option<u16>,
    pub username: String,
}

/// One live connection; clones share it.
#[derive(Clone)]
pub struct RemoteConn {
    vfs: Arc<dyn Vfs>,
    info: Arc<RemoteInfo>,
}

impl std::fmt::Debug for RemoteConn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteConn")
            .field("info", &self.info)
            .finish()
    }
}

impl RemoteConn {
    pub fn new(vfs: Arc<dyn Vfs>, info: RemoteInfo) -> Self {
        Self {
            vfs,
            info: Arc::new(info),
        }
    }

    /// The adapter serving this connection.
    pub fn vfs(&self) -> Arc<dyn Vfs> {
        Arc::clone(&self.vfs)
    }

    pub fn info(&self) -> &RemoteInfo {
        &self.info
    }

    /// Identity of the connection (equal for clones of one connection).
    pub fn id(&self) -> usize {
        Arc::as_ptr(&self.info).addr()
    }

    /// `true` when both are the same connection (a move between them can
    /// be a server-side rename).
    pub fn is_same_server(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.info, &other.info)
    }

    /// `user@host` as shown in titles and prompts.
    pub fn user_at_host(&self) -> String {
        user_at_host(&self.info.username, &self.info.host)
    }
}

/// `user@host`, or just `host` without a user.
pub fn user_at_host(user: &str, host: &str) -> String {
    if user.is_empty() {
        host.to_string()
    } else {
        format!("{user}@{host}")
    }
}

impl std::ops::Deref for RemoteConn {
    type Target = dyn Vfs;

    fn deref(&self) -> &Self::Target {
        self.vfs.as_ref()
    }
}

impl From<SharedSshClient> for RemoteConn {
    fn from(client: SharedSshClient) -> Self {
        let ssh = client.info();
        let info = RemoteInfo {
            kind: RemoteKind::Sftp,
            host: ssh.host.clone(),
            port: Some(ssh.port),
            username: ssh.username.clone(),
        };
        Self::new(Arc::new(client), info)
    }
}

/// The filesystem on one side of a transfer: the connection when there is
/// one, the local disk otherwise.
pub fn endpoint_vfs(conn: &Option<RemoteConn>) -> Arc<dyn Vfs> {
    match conn {
        Some(conn) => conn.vfs(),
        None => Arc::new(crate::fs::vfs::LocalVfs),
    }
}

/// `is_dir` on either side of a transfer: remote when `conn` is set.
pub fn is_dir_on(path: &std::path::Path, conn: &Option<RemoteConn>) -> bool {
    match conn {
        Some(conn) => conn.metadata(path).is_ok_and(|e| e.is_dir),
        None => path.is_dir(),
    }
}
