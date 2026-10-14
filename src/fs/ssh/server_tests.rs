//! The Vfs contract and a resumed transfer against a real SSH server
//! (opt-in: `PAIREE_SFTP_TEST=1`, see CONTRIBUTING.md). The server is the
//! container of `docker/docker-compose.yml` unless `PAIREE_SFTP_HOST`,
//! `PAIREE_SFTP_PORT`, `PAIREE_SFTP_USER` and `PAIREE_SFTP_PASSWORD` name
//! another one. Every fixture works in a folder of its own below `/tmp`,
//! removed afterwards.

use super::{SharedSshClient, SshLogin};
use crate::fs::vfs::Vfs;
use crate::fs::vfs::contract::{Fixture, TREE, vfs_contract};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

fn setting(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

/// Trusts the key the test server presents, in a known_hosts file of the
/// tests (the user's own file is never read or written).
fn trust_server(host: &str, port: u16) -> PathBuf {
    let tcp = std::net::TcpStream::connect((host, port)).expect("reach the SFTP test server");
    let mut session = ssh2::Session::new().unwrap();
    session.set_tcp_stream(tcp);
    session.handshake().unwrap();
    let (key, kind) = session.host_key().expect("a host key");
    let mut known = session.known_hosts().unwrap();
    known
        .add(&format!("[{host}]:{port}"), key, "pairee test", kind.into())
        .unwrap();
    let path = tempfile::tempdir().unwrap().keep().join("known_hosts");
    known
        .write_file(&path, ssh2::KnownHostFileKind::OpenSSH)
        .unwrap();
    path
}

/// A logged-in client of the test server.
pub(crate) fn client() -> SharedSshClient {
    let host = setting("PAIREE_SFTP_HOST", "127.0.0.1");
    let port = setting("PAIREE_SFTP_PORT", "2222");
    let port_number: u16 = port.parse().expect("PAIREE_SFTP_PORT");
    super::connection::TEST_KNOWN_HOSTS.get_or_init(|| trust_server(&host, port_number));
    let user = setting("PAIREE_SFTP_USER", "pairee");
    let password = setting("PAIREE_SFTP_PASSWORD", "pairee");
    SharedSshClient::connect(&SshLogin {
        host: &host,
        port: port.parse().expect("PAIREE_SFTP_PORT"),
        username: &user,
        password: Some(&password),
        key_path: None,
        timeout: Duration::from_secs(20),
        ask: &crate::credential_prompt::nobody,
    })
    .expect("connect to the SFTP test server (is docker compose up?)")
}

/// A folder of its own on the server, removed when dropped.
pub(crate) struct ServerFolder {
    pub vfs: Arc<dyn Vfs>,
    pub path: PathBuf,
}

impl ServerFolder {
    pub(crate) fn new() -> Self {
        let vfs: Arc<dyn Vfs> = Arc::new(client());
        let name = tempfile::Builder::new()
            .prefix("pairee-sftp-")
            .tempdir()
            .unwrap()
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let path = Path::new("/tmp").join(name);
        vfs.mkdir(&path).unwrap();
        Self { vfs, path }
    }
}

impl Drop for ServerFolder {
    fn drop(&mut self) {
        let _ = self.vfs.remove_all(&self.path);
    }
}

fn fixture() -> Fixture {
    let folder = ServerFolder::new();
    for (rel, body) in TREE {
        let path = folder.path.join(rel);
        if let Some(parent) = path.parent() {
            folder.vfs.mkdir_all(parent);
        }
        folder.vfs.write_file(&path, &mut &body[..]).unwrap();
    }
    Fixture {
        vfs: Arc::clone(&folder.vfs),
        root: folder.path.clone(),
        _dir: tempfile::tempdir().unwrap(),
        empty_folders: true,
        _hold: Some(Box::new(folder)),
    }
}

vfs_contract!(sftp_server, super::fixture);
