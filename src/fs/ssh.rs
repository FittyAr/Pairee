use crate::fs::entry::FileEntry;
use crate::app::state::SortField;
use anyhow::{Context, Result};
use std::net::{TcpStream, SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use ssh2::{Session, Sftp};

pub struct SshClient {
    pub host: String,
    pub port: u16,
    pub username: String,
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
            .context("Failed to resolve host address")?
            .collect::<Vec<SocketAddr>>();

        if socket_addrs.is_empty() {
            anyhow::bail!("No socket addresses found for host: {}", host);
        }

        // Connect with a 5 second timeout
        let stream = TcpStream::connect_timeout(&socket_addrs[0], Duration::from_secs(5))
            .context("Failed to connect to host (connection timeout)")?;

        let mut sess = Session::new().context("Failed to create SSH session")?;
        sess.set_tcp_stream(stream);
        sess.handshake().context("SSH handshake failed")?;

        let mut authenticated = false;

        // Try key authentication if provided
        if let Some(kp) = key_path {
            if !kp.trim().is_empty() {
                let path = Path::new(kp);
                if path.exists() {
                    sess.userauth_pubkey_file(username, None, path, password)
                        .context("SSH key authentication failed")?;
                    authenticated = true;
                }
            }
        }

        // Try password authentication if key failed/not provided
        if !authenticated {
            if let Some(pass) = password {
                sess.userauth_password(username, pass)
                    .context("SSH password authentication failed")?;
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
                    if sess.userauth_pubkey_file(username, None, path, None).is_ok() {
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
            anyhow::bail!("Authentication failed (check credentials)");
        }

        let sftp = sess.sftp().context("Failed to initialize SFTP channel")?;

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
        let client = self.0.lock().map_err(|_| anyhow::anyhow!("SshClient mutex poisoned"))?;
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
                    let modified = stat.mtime.map(|mtime| {
                        SystemTime::UNIX_EPOCH + Duration::from_secs(mtime)
                    });

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
            Err(e) => anyhow::bail!("Failed to read SFTP directory: {}", e),
        };

        entries.append(&mut read_entries);

        // 3. Sort entries (pinning ".." first)
        if matches!(sort_field, SortField::Unsorted) {
            if let Some(pos) = entries.iter().position(|e| e.name == "..") {
                let dotdot = entries.remove(pos);
                entries.insert(0, dotdot);
            }
        } else {
            entries.sort_by(|a, b| {
                if a.name == ".." {
                    return std::cmp::Ordering::Less;
                }
                if b.name == ".." {
                    return std::cmp::Ordering::Greater;
                }

                let dir_order = if matches!(sort_field, SortField::Extension) {
                    std::cmp::Ordering::Equal
                } else {
                    match (a.is_dir, b.is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => std::cmp::Ordering::Equal,
                    }
                };

                if dir_order != std::cmp::Ordering::Equal {
                    return if sort_reverse {
                        dir_order.reverse()
                    } else {
                        dir_order
                    };
                }

                let name_ord = match sort_field {
                    SortField::Name => {
                        if treat_digits_as_numbers {
                            cmp_natural(&a.name, &b.name, case_sensitive_sort)
                        } else {
                            cmp_standard(&a.name, &b.name, case_sensitive_sort)
                        }
                    }
                    SortField::Extension => {
                        let ext_a = entry_sort_key_ext(a);
                        let ext_b = entry_sort_key_ext(b);
                        let ext_ord = if treat_digits_as_numbers {
                            cmp_natural(&ext_a, &ext_b, case_sensitive_sort)
                        } else {
                            cmp_standard(&ext_a, &ext_b, case_sensitive_sort)
                        };
                        if ext_ord == std::cmp::Ordering::Equal {
                            if treat_digits_as_numbers {
                                cmp_natural(&a.name, &b.name, case_sensitive_sort)
                            } else {
                                cmp_standard(&a.name, &b.name, case_sensitive_sort)
                            }
                        } else {
                            ext_ord
                        }
                    }
                    SortField::Size => {
                        if a.is_dir && b.is_dir {
                            cmp_standard(&a.name, &b.name, case_sensitive_sort)
                        } else {
                            a.size.cmp(&b.size)
                        }
                    }
                    SortField::Date => {
                        let t_a = a.modified.map(|t| {
                            t.duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs()
                        });
                        let t_b = b.modified.map(|t| {
                            t.duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs()
                        });
                        t_a.cmp(&t_b)
                    }
                    SortField::Unsorted => std::cmp::Ordering::Equal,
                };

                if sort_reverse {
                    name_ord.reverse()
                } else {
                    name_ord
                }
            });
        }

        Ok(entries)
    }

    pub fn create_dir(&self, path: &Path) -> Result<()> {
        let client = self.0.lock().map_err(|_| anyhow::anyhow!("SshClient mutex poisoned"))?;
        client.sftp.mkdir(path, 0o755)?;
        Ok(())
    }

    pub fn delete_recursive(&self, path: &Path) -> Result<()> {
        let client = self.0.lock().map_err(|_| anyhow::anyhow!("SshClient mutex poisoned"))?;
        
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
        let client = self.0.lock().map_err(|_| anyhow::anyhow!("SshClient mutex poisoned"))?;
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
        let client = self.0.lock().map_err(|_| anyhow::anyhow!("SshClient mutex poisoned"))?;
        client.sftp.rename(src, dst, None)?;
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sorting helpers
// ─────────────────────────────────────────────────────────────────────────────

fn entry_sort_key_ext(entry: &FileEntry) -> String {
    if entry.is_dir {
        String::new()
    } else {
        Path::new(&entry.name)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    }
}

fn cmp_natural(a: &str, b: &str, case_sensitive: bool) -> std::cmp::Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(&ca), Some(&cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let mut num_a: u64 = 0;
                    while let Some(&c) = a_chars.peek() {
                        if c.is_ascii_digit() {
                            num_a = num_a
                                .saturating_mul(10)
                                .saturating_add(c.to_digit(10).unwrap() as u64);
                            a_chars.next();
                        } else {
                            break;
                        }
                    }
                    let mut num_b: u64 = 0;
                    while let Some(&c) = b_chars.peek() {
                        if c.is_ascii_digit() {
                            num_b = num_b
                                .saturating_mul(10)
                                .saturating_add(c.to_digit(10).unwrap() as u64);
                            b_chars.next();
                        } else {
                            break;
                        }
                    }
                    match num_a.cmp(&num_b) {
                        std::cmp::Ordering::Equal => continue,
                        ord => return ord,
                    }
                } else {
                    let mut char_a = a_chars.next().unwrap();
                    let mut char_b = b_chars.next().unwrap();
                    if !case_sensitive {
                        char_a = char_a.to_lowercase().next().unwrap_or(char_a);
                        char_b = char_b.to_lowercase().next().unwrap_or(char_b);
                    }
                    match char_a.cmp(&char_b) {
                        std::cmp::Ordering::Equal => continue,
                        ord => return ord,
                    }
                }
            }
        }
    }
}

fn cmp_standard(a: &str, b: &str, case_sensitive: bool) -> std::cmp::Ordering {
    if case_sensitive {
        a.cmp(b)
    } else {
        a.to_lowercase().cmp(&b.to_lowercase())
    }
}
