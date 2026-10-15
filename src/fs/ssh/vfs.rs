//! The SFTP adapter of the panel source port. The session is locked per
//! call, so other remote operations can interleave with long walks.

use super::SharedSshClient;
use super::sftp_ops::{sftp_attrs, sftp_entry};
use crate::fs::FileEntry;
use crate::fs::attrs::{AttrChange, FileAttrs};
use crate::fs::file_id::FileIdentity;
use crate::fs::list::ListOptions;
use crate::fs::vfs::{Capabilities, Vfs, VfsEntry, is_plain_name, panel_listing};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::SystemTime;

/// An SFTP error as an I/O error whose kind callers can test (a missing
/// file is `NotFound`, as on the other adapters).
fn io_err(e: ssh2::Error) -> io::Error {
    // SSH_FX_* status codes of the SFTP protocol.
    let kind = match e.code() {
        ssh2::ErrorCode::SFTP(2 | 10) => io::ErrorKind::NotFound,
        ssh2::ErrorCode::SFTP(3) => io::ErrorKind::PermissionDenied,
        ssh2::ErrorCode::SFTP(11) => io::ErrorKind::AlreadyExists,
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, e)
}

/// A remote file whose `flush` completes it: the data is flushed, synced
/// where the server has the `fsync` extension, and the handle closed, with
/// every error returned (a failed close is a failed upload, as a dropped
/// handle would hide it). Nothing can be written afterwards.
struct SyncOnFlush(Option<ssh2::File>);

/// The result of the optional `fsync` request: a server that answers with an
/// SFTP status (no such extension, `SSH_FX_OP_UNSUPPORTED` or a plain
/// failure) is skipped; a broken session is an error.
fn sync_result(result: Result<(), ssh2::Error>) -> io::Result<()> {
    match result {
        Err(e) if matches!(e.code(), ssh2::ErrorCode::SFTP(_)) => Ok(()),
        other => other.map_err(io_err),
    }
}

impl SyncOnFlush {
    fn open(&mut self) -> io::Result<&mut ssh2::File> {
        self.0
            .as_mut()
            .ok_or_else(|| io::Error::other("upload already finished"))
    }
}

impl Write for SyncOnFlush {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.open()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let Some(mut file) = self.0.take() else {
            return Ok(());
        };
        file.flush()?;
        sync_result(file.fsync())?;
        file.close().map_err(io_err)
    }
}

/// The attributes a `setstat` changes: the permission bits and/or the
/// times (access and modification alike); everything else is left alone.
fn setstat_of(perm: Option<u32>, secs: Option<u64>) -> ssh2::FileStat {
    ssh2::FileStat {
        size: None,
        uid: None,
        gid: None,
        perm,
        atime: secs,
        mtime: secs,
    }
}

/// libssh2 codes that say the connection is gone (socket send / receive
/// failures, the server hanging up, an unusable socket). Timeouts are not
/// among them: the operation may still be running on the server.
const CONNECTION_LOST: [i32; 5] = [-2, -7, -13, -43, -45];

/// Whether `error` means the session died (not that the request failed).
pub(super) fn connection_lost(error: &ssh2::Error) -> bool {
    matches!(error.code(), ssh2::ErrorCode::Session(code) if CONNECTION_LOST.contains(&code))
}

impl SharedSshClient {
    /// Runs `op` on the SFTP channel. When the connection turned out to be
    /// dead, it is opened again once (with the saved login, never asking
    /// the user) and `op` retried, so a server that dropped an idle
    /// session does not leave the panel unusable.
    fn with_sftp<T>(&self, op: impl Fn(&ssh2::Sftp) -> Result<T, ssh2::Error>) -> io::Result<T> {
        let first = op(&self.lock().sftp);
        match first {
            Err(e) if connection_lost(&e) && self.reconnect().is_ok() => {
                op(&self.lock().sftp).map_err(io_err)
            }
            other => other.map_err(io_err),
        }
    }
}

impl Vfs for SharedSshClient {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            local_tools: false,
            ..Capabilities::FULL
        }
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
        let items = self.with_sftp(|sftp| sftp.readdir(dir))?;
        // The path is the folder joined with the name the server sent: only
        // plain names (no `..`, no `\` on Windows) stay inside the folder.
        let plain = |path: &Path| {
            path.strip_prefix(dir)
                .is_ok_and(|rel| is_plain_name(&rel.to_string_lossy()))
        };
        Ok(items
            .into_iter()
            .filter(|(path, _)| plain(path))
            .filter_map(|(path, stat)| sftp_entry(path, &stat))
            .collect())
    }

    fn stat(&self, path: &Path) -> io::Result<VfsEntry> {
        let stat = self.with_sftp(|sftp| sftp.lstat(path))?;
        sftp_entry(path.to_path_buf(), &stat)
            .or_else(|| {
                // The root has no file name.
                stat.is_dir()
                    .then(|| VfsEntry::dir(String::new(), path.to_path_buf()))
            })
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    fn metadata(&self, path: &Path) -> io::Result<VfsEntry> {
        let stat = self.with_sftp(|sftp| sftp.stat(path))?;
        let mut entry = sftp_entry(path.to_path_buf(), &stat)
            .unwrap_or_else(|| VfsEntry::dir(String::new(), path.to_path_buf()));
        // `stat` describes the target; the path itself may still be a link.
        entry.is_symlink = self.stat(path).is_ok_and(|e| e.is_symlink);
        Ok(entry)
    }

    /// SFTP has no inodes: the canonical (`realpath`) path identifies.
    fn identity(&self, path: &Path) -> Option<FileIdentity> {
        let real = self.with_sftp(|sftp| sftp.realpath(path)).ok()?;
        Some(FileIdentity::Path(real))
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn Read + Send>> {
        let file = self.with_sftp(|sftp| sftp.open(path))?;
        Ok(Box::new(file))
    }

    fn open_read_at(&self, path: &Path, offset: u64) -> io::Result<Box<dyn Read + Send>> {
        let mut file = self.with_sftp(|sftp| sftp.open(path))?;
        file.seek(SeekFrom::Start(offset))?;
        Ok(Box::new(file))
    }

    fn create_file(&self, path: &Path) -> io::Result<Box<dyn Write + Send>> {
        let file = self.with_sftp(|sftp| sftp.create(path))?;
        Ok(Box::new(SyncOnFlush(Some(file))))
    }

    /// Opened for writing without truncation, positioned after the last
    /// byte (SFTP writes carry their offset, so no server-side append mode
    /// is needed).
    fn append_file(&self, path: &Path) -> io::Result<Box<dyn Write + Send>> {
        let size = self.with_sftp(|sftp| sftp.stat(path))?.size.unwrap_or(0);
        let mut file = self.with_sftp(|sftp| {
            sftp.open_mode(path, ssh2::OpenFlags::WRITE, 0o644, ssh2::OpenType::File)
        })?;
        file.seek(SeekFrom::Start(size))?;
        Ok(Box::new(SyncOnFlush(Some(file))))
    }

    /// SFTP v3 servers (OpenSSH) refuse to rename over an existing file.
    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        crate::fs::vfs::replace_by_moving_aside(self, from, to)
    }

    /// `setstat` with the time (access time set alike) and permission bits.
    fn set_metadata(
        &self,
        path: &Path,
        modified: Option<SystemTime>,
        mode: Option<u32>,
    ) -> io::Result<()> {
        let secs = modified
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let perm = mode.map(|m| m & 0o7777);
        self.with_sftp(|sftp| sftp.setstat(path, setstat_of(perm, secs)))
    }

    fn mkdir(&self, path: &Path) -> io::Result<()> {
        self.with_sftp(|sftp| sftp.mkdir(path, 0o755))
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.with_sftp(|sftp| sftp.unlink(path))
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.with_sftp(|sftp| sftp.rmdir(path))
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.with_sftp(|sftp| sftp.rename(from, to, None))
    }

    fn attributes(&self, path: &Path) -> io::Result<FileAttrs> {
        let stat = self.with_sftp(|sftp| sftp.stat(path))?;
        Ok(sftp_attrs(path, &stat))
    }

    /// `chmod` through SFTP `setstat` (only the permission bits are sent).
    fn set_attributes(&self, path: &Path, change: AttrChange) -> io::Result<()> {
        let current = self.with_sftp(|sftp| sftp.stat(path))?.perm.unwrap_or(0);
        let perm = Some(change.resulting_mode(current));
        self.with_sftp(|sftp| sftp.setstat(path, setstat_of(perm, None)))
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
