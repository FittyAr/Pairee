//! The SFTP adapter of the panel source port. The session is locked per
//! call, so other remote operations can interleave with long walks.

use super::SharedSshClient;
use super::sftp_ops::{sftp_attrs, sftp_entry};
use crate::fs::FileEntry;
use crate::fs::attrs::{AttrChange, FileAttrs};
use crate::fs::list::ListOptions;
use crate::fs::vfs::{Capabilities, Vfs, VfsEntry, panel_listing};
use std::io::{self, Read};
use std::path::Path;

fn io_err(e: ssh2::Error) -> io::Error {
    io::Error::other(e)
}

impl Vfs for SharedSshClient {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            local_tools: false,
            ..Capabilities::FULL
        }
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
        let items = self.lock().sftp.readdir(dir).map_err(io_err)?;
        Ok(items
            .into_iter()
            .filter_map(|(path, stat)| sftp_entry(path, &stat))
            .collect())
    }

    fn stat(&self, path: &Path) -> io::Result<VfsEntry> {
        let stat = self.lock().sftp.lstat(path).map_err(io_err)?;
        sftp_entry(path.to_path_buf(), &stat)
            .or_else(|| {
                // The root has no file name.
                stat.is_dir()
                    .then(|| VfsEntry::dir(String::new(), path.to_path_buf()))
            })
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn Read + Send>> {
        let file = self.lock().sftp.open(path).map_err(io_err)?;
        Ok(Box::new(file))
    }

    fn write_file(&self, path: &Path, data: &mut dyn Read) -> io::Result<()> {
        let mut file = self.lock().sftp.create(path).map_err(io_err)?;
        io::copy(data, &mut file).map(|_| ())
    }

    fn mkdir(&self, path: &Path) -> io::Result<()> {
        self.lock().sftp.mkdir(path, 0o755).map_err(io_err)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.lock().sftp.unlink(path).map_err(io_err)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.lock().sftp.rmdir(path).map_err(io_err)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.lock().sftp.rename(from, to, None).map_err(io_err)
    }

    fn attributes(&self, path: &Path) -> io::Result<FileAttrs> {
        let stat = self.lock().sftp.stat(path).map_err(io_err)?;
        Ok(sftp_attrs(path, &stat))
    }

    /// `chmod` through SFTP `setstat` (only the permission bits are sent).
    fn set_attributes(&self, path: &Path, change: AttrChange) -> io::Result<()> {
        let client = self.lock();
        let current = client.sftp.stat(path).map_err(io_err)?.perm.unwrap_or(0);
        let stat = ssh2::FileStat {
            size: None,
            uid: None,
            gid: None,
            perm: Some(change.resulting_mode(current)),
            atime: None,
            mtime: None,
        };
        client.sftp.setstat(path, stat).map_err(io_err)
    }

    /// Lists with the error text panels show for remote folders; folders
    /// are never sorted by extension remotely.
    fn read_panel(&self, dir: &Path, opts: &ListOptions) -> anyhow::Result<Vec<FileEntry>> {
        let children = self.list(dir).map_err(|e| {
            anyhow::anyhow!(
                crate::config::localization::t("error_ssh_read_dir_failed")
                    .replace("{}", &e.to_string())
            )
        })?;
        let opts = ListOptions {
            folder_by_ext: false,
            ..opts.clone()
        };
        Ok(panel_listing(dir, children, &opts))
    }
}
