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
        let client = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!(t("error_mutex_poisoned")))?;

        // Let's check if the path is a directory or a file
        let metadata = client.sftp.stat(path);
        if let Ok(stat) = metadata {
            if stat.is_dir() {
                // Read dir contents and recursively delete them
                let entries = client.sftp.readdir(path)?;
                for (entry_path, entry_stat) in entries {
                    let name = entry_path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if name == "." || name == ".." {
                        continue;
                    }
                    if entry_stat.is_dir() {
                        drop(client);
                        self.delete_recursive(&entry_path)?;
                        return self.delete_recursive(path); // retry original
                    } else {
                        client.sftp.unlink(&entry_path)?;
                    }
                }
                client.sftp.rmdir(path)?;
            } else {
                client.sftp.unlink(path)?;
            }
        } else {
            // Stat failed or doesn't exist, try to delete file anyway
            let _ = client.sftp.unlink(path);
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
