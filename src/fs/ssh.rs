use crate::app::state::SortField;
use crate::config::localization::t;
use crate::fs::entry::FileEntry;
use anyhow::{Context, Result};
use ssh2::{Session, Sftp};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

pub struct SshClient {
    pub host: String,
    pub port: u16,
    pub username: String,
    #[allow(dead_code)]
    pub session: Session,
    pub sftp: Sftp,
}

#[derive(Clone)]
pub struct SharedSshClient(pub Arc<Mutex<SshClient>>);

impl std::fmt::Debug for SharedSshClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(client) = self.0.lock() {
            f.debug_struct("SharedSshClient")
                .field("host", &client.host)
                .field("port", &client.port)
                .field("username", &client.username)
                .finish()
        } else {
            f.write_str("SharedSshClient(Locked)")
        }
    }
}

impl SharedSshClient {
    pub fn is_same_server(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub fn connect(
        host: &str,
        port: u16,
        username: &str,
        password: Option<&str>,
        key_path: Option<&str>,
    ) -> Result<Self> {
        let addr = format!("{}:{}", host, port);
        let socket_addrs = addr
            .to_socket_addrs()
            .context(t("error_ssh_resolve_host"))?
            .collect::<Vec<SocketAddr>>();

        if socket_addrs.is_empty() {
            anyhow::bail!(t("error_ssh_no_socket_addr").replace("{}", host));
        }

        // Connect with a 5 second timeout
        let stream = TcpStream::connect_timeout(&socket_addrs[0], Duration::from_secs(5))
            .context(t("error_ssh_connect_timeout"))?;

        let mut sess = Session::new().context(t("error_ssh_create_session"))?;
        sess.set_tcp_stream(stream);
        sess.handshake().context(t("error_ssh_handshake_failed"))?;

        // ── Host key verification ─────────────────────────────────────────
        // Without this check, an attacker on the network can MITM the SSH
        // connection and capture the user's credentials or tamper with the
        // session. ssh2's `Session::handshake()` does NOT verify host keys
        // by default; we must compare the server's key against a known_hosts
        // file ourselves. The `known_hosts_path` defaults to the user's
        // OpenSSH known_hosts file.
        let known_hosts_path = known_hosts_path();
        let mut known_hosts = sess
            .known_hosts()
            .context("Failed to allocate SSH known_hosts handle")?;
        let kh_loaded = if let Some(ref p) = known_hosts_path {
            if p.exists() {
                match known_hosts.read_file(p, ssh2::KnownHostFileKind::OpenSSH) {
                    Ok(_) => true,
                    Err(e) => {
                        log::warn!(
                            "Failed to read known_hosts file {:?}: {} — host key \
                             verification is disabled for this connection; \
                             the session is vulnerable to MITM.",
                            p, e
                        );
                        false
                    }
                }
            } else {
                // First connection: try to create the parent directory so
                // subsequent connections can verify the server. The very
                // first connection will be refused because we have no
                // trusted key yet.
                if let Some(parent) = p.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                log::warn!(
                    "known_hosts file {:?} does not exist; SSH host key \
                     verification is disabled for this connection. Add the \
                     server's host key to the file to enable verification.",
                    p
                );
                false
            }
        } else {
            log::warn!(
                "Could not determine a known_hosts file path; SSH host key \
                 verification is disabled for this connection."
            );
            false
        };

        if !kh_loaded {
            anyhow::bail!(
                "Refusing SSH connection to {}:{} because host key \
                 verification is not available. Add the server's host key to \
                 your known_hosts file and try again.",
                host,
                port
            );
        }

        let hostkey = sess
            .host_key()
            .ok_or_else(|| anyhow::anyhow!("SSH handshake returned no host key"))?;
        // `host_key()` returns `(raw_key_bytes, key_type)`. The known_hosts
        // check only consumes the raw bytes; the key type is metadata that
        // libssh2 also extracts from the key during the check.
        let (key_bytes, _key_type) = hostkey;
        match known_hosts.check_port(host, port, key_bytes) {
            ssh2::CheckResult::Match => {}
            ssh2::CheckResult::Mismatch => {
                anyhow::bail!(
                    "SSH host key for {}:{} does NOT match the key in known_hosts. \
                     This may indicate a man-in-the-middle attack.",
                    host,
                    port
                );
            }
            ssh2::CheckResult::NotFound => {
                // First time we see this key. Safe-by-default: refuse the
                // connection. The user must explicitly trust the new key
                // by adding it to their known_hosts file. This prevents
                // the classic SSH "first-connection MITM" attack.
                anyhow::bail!(
                    "SSH host key for {}:{} is not in known_hosts. Refusing to \
                     connect. Add the server's host key to {:?} and try again.",
                    host,
                    port,
                    known_hosts_path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                );
            }
            ssh2::CheckResult::Failure => {
                anyhow::bail!("SSH host key check failed for {}:{}", host, port);
            }
        }

        let mut authenticated = false;

        // Try key authentication if provided
        if let Some(kp) = key_path {
            if !kp.trim().is_empty() {
                let path = Path::new(kp);
                if path.exists() {
                    sess.userauth_pubkey_file(username, None, path, password)
                        .context(t("error_ssh_key_auth_failed"))?;
                    authenticated = true;
                }
            }
        }

        // Try password authentication if key failed/not provided
        if !authenticated {
            if let Some(pass) = password {
                sess.userauth_password(username, pass)
                    .context(t("error_ssh_password_auth_failed"))?;
                authenticated = true;
            }
        }

        // Try default keys if still not authenticated
        if !authenticated {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            let keys = vec![
                format!("{}/.ssh/id_rsa", home),
                format!("{}/.ssh/id_ed25519", home),
            ];
            for key in keys {
                let path = Path::new(&key);
                if path.exists() {
                    if sess
                        .userauth_pubkey_file(username, None, path, None)
                        .is_ok()
                    {
                        authenticated = true;
                        break;
                    }
                }
            }
        }

        // Try agent if still not authenticated
        if !authenticated {
            if sess.userauth_agent(username).is_ok() {
                authenticated = true;
            }
        }

        if !authenticated {
            anyhow::bail!(t("error_ssh_auth_failed"));
        }

        let sftp = sess.sftp().context(t("error_ssh_init_sftp"))?;

        Ok(Self(Arc::new(Mutex::new(SshClient {
            host: host.to_string(),
            port,
            username: username.to_string(),
            session: sess,
            sftp,
        }))))
    }

    pub fn read_directory(
        &self,
        path: &Path,
        show_hidden: bool,
        case_sensitive_sort: bool,
        treat_digits_as_numbers: bool,
        sort_field: SortField,
        sort_reverse: bool,
        show_dotdot_in_root_folders: bool,
    ) -> Result<Vec<FileEntry>> {
        let client = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!(t("error_mutex_poisoned")))?;
        let mut entries = Vec::new();

        // 1. Add ".." parent directory entry
        let path_str = path.to_string_lossy().to_string();
        let is_root = path_str == "/" || path_str.is_empty();
        if !is_root {
            let parent = path.parent().unwrap_or(Path::new("/"));
            entries.push(FileEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                size: 0,
                is_dir: true,
                is_symlink: false,
                modified: None,
            });
        } else if show_dotdot_in_root_folders {
            entries.push(FileEntry {
                name: "..".to_string(),
                path: path.to_path_buf(),
                size: 0,
                is_dir: true,
                is_symlink: false,
                modified: None,
            });
        }

        // 2. Read SFTP directory contents
        let read_res = client.sftp.readdir(path);
        let mut read_entries = match read_res {
            Ok(items) => {
                let mut mapped = Vec::new();
                for (path_buf, stat) in items {
                    let name = path_buf
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();

                    if name.is_empty() || name == "." || name == ".." {
                        continue;
                    }

                    if !show_hidden && name.starts_with('.') {
                        continue;
                    }

                    let is_dir = stat.is_dir();
                    let is_symlink = stat.file_type().is_symlink();
                    let size = stat.size.unwrap_or(0);
                    let modified = stat
                        .mtime
                        .map(|mtime| SystemTime::UNIX_EPOCH + Duration::from_secs(mtime));

                    mapped.push(FileEntry {
                        name,
                        path: path_buf,
                        size,
                        is_dir,
                        is_symlink,
                        modified,
                    });
                }
                mapped
            }
            Err(e) => anyhow::bail!(t("error_ssh_read_dir_failed").replace("{}", &e.to_string())),
        };

        entries.append(&mut read_entries);

        // 3. Sort entries (pinning ".." first) using the centralized sort_entries helper
        crate::fs::list::sort_entries(
            &mut entries,
            sort_field,
            sort_reverse,
            case_sensitive_sort,
            treat_digits_as_numbers,
            false,
        );

        Ok(entries)
    }

    pub fn create_dir(&self, path: &Path) -> Result<()> {
        let client = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!(t("error_mutex_poisoned")))?;
        client.sftp.mkdir(path, 0o755)?;
        Ok(())
    }

    pub fn delete_recursive(&self, path: &Path) -> Result<()> {
        // Use an explicit work-stack so we never re-enter this function
        // recursively on the same path. The previous implementation used
        // `return self.delete_recursive(path)` after deleting a child dir,
        // which both abandoned the rest of the parent's entries AND could
        // recurse indefinitely on deeply nested trees.
        let mut stack: Vec<PathBuf> = vec![path.to_path_buf()];
        while let Some(current) = stack.pop() {
            // Take the lock, stat, release. We can't hold the lock across
            // the recursive calls because each call needs its own lock guard.
            let (is_dir, children) = {
                let client = self
                    .0
                    .lock()
                    .map_err(|_| anyhow::anyhow!(t("error_mutex_poisoned")))?;
                match client.sftp.stat(&current) {
                    Ok(stat) => {
                        if stat.is_dir() {
                            let kids = client.sftp.readdir(&current)?;
                            let mut names: Vec<PathBuf> = Vec::with_capacity(kids.len());
                            for (entry_path, entry_stat) in kids {
                                let name = entry_path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                if name == "." || name == ".." || name.is_empty() {
                                    continue;
                                }
                                if entry_stat.is_dir() {
                                    stack.push(entry_path);
                                } else {
                                    names.push(entry_path);
                                }
                            }
                            (true, Some(names))
                        } else {
                            (false, None)
                        }
                    }
                    Err(_) => (false, None),
                }
            };

            if is_dir {
                // Push children that need further recursion back onto the stack.
                if let Some(kids) = children {
                    for k in kids {
                        // Files are unlinked inline; dirs are walked.
                        let client = self
                            .0
                            .lock()
                            .map_err(|_| anyhow::anyhow!(t("error_mutex_poisoned")))?;
                        client.sftp.unlink(&k)?;
                    }
                }
                // After all entries are removed, the directory can be rmdir'd.
                let client = self
                    .0
                    .lock()
                    .map_err(|_| anyhow::anyhow!(t("error_mutex_poisoned")))?;
                client.sftp.rmdir(&current)?;
            } else {
                // File (or stat failed — try unlink, ignore errors).
                let client = self
                    .0
                    .lock()
                    .map_err(|_| anyhow::anyhow!(t("error_mutex_poisoned")))?;
                let _ = client.sftp.unlink(&current);
            }
        }
        Ok(())
    }

    pub fn walk_dir(&self, root: &Path) -> Result<Vec<(PathBuf, bool, u64)>> {
        let client = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!(t("error_mutex_poisoned")))?;
        let mut results = Vec::new();
        let mut to_visit = vec![root.to_path_buf()];

        while let Some(dir) = to_visit.pop() {
            if let Ok(entries) = client.sftp.readdir(&dir) {
                for (path_buf, stat) in entries {
                    let name = path_buf
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if name == "." || name == ".." || name.is_empty() {
                        continue;
                    }
                    let is_dir = stat.is_dir();
                    let size = stat.size.unwrap_or(0);
                    results.push((path_buf.clone(), is_dir, size));
                    if is_dir {
                        to_visit.push(path_buf);
                    }
                }
            }
        }
        Ok(results)
    }

    pub fn rename_move(&self, src: &Path, dst: &Path) -> Result<()> {
        let client = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!(t("error_mutex_poisoned")))?;
        client.sftp.rename(src, dst, None)?;
        Ok(())
    }
}

/// Returns the platform-specific location of the OpenSSH `known_hosts` file
/// used for SSH host key verification.
fn known_hosts_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // %USERPROFILE%\.ssh\known_hosts — matches OpenSSH for Windows and
        // OpenSSH-for-Windows-Portable default locations.
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return Some(
                std::path::PathBuf::from(profile)
                    .join(".ssh")
                    .join("known_hosts"),
            );
        }
        None
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return Some(
                std::path::PathBuf::from(home)
                    .join(".ssh")
                    .join("known_hosts"),
            );
        }
        None
    }
}
