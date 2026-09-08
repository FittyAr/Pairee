//! SSH client and shared handle for remote operations.

mod connection;
mod sftp_ops;

use crate::app::state::SortField;
use crate::config::localization::t;
use crate::fs::entry::FileEntry;
use anyhow::Result;
use ssh2::{Session, Sftp};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
        let session = connection::open_session(host, port, username, password, key_path)?;
        let sftp = session
            .sftp()
            .map_err(|e| anyhow::anyhow!("{}: {}", t("error_ssh_init_sftp"), e))?;

        Ok(Self(Arc::new(Mutex::new(SshClient {
            host: host.to_string(),
            port,
            username: username.to_string(),
            session,
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
        sftp_ops::read_directory(
            &client.sftp,
            path,
            show_hidden,
            case_sensitive_sort,
            treat_digits_as_numbers,
            sort_field,
            sort_reverse,
            show_dotdot_in_root_folders,
        )
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
        sftp_ops::delete_recursive(&client.sftp, path)
    }

    pub fn walk_dir(&self, root: &Path) -> Result<Vec<(PathBuf, bool, u64)>> {
        let client = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!(t("error_mutex_poisoned")))?;
        sftp_ops::walk_dir(&client.sftp, root)
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
