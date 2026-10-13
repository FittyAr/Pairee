//! The FTP / FTPS adapter. Listings use `MLSD` (falling back to `LIST`,
//! POSIX or DOS style), reads and writes stream on connections of their
//! own (`REST` resumes, `APPE` appends), times are set with `MFMT` and
//! modes with `SITE CHMOD` where the server has them. `stat` asks `MLST`
//! when the server answers it and reads the parent's listing otherwise.

mod session;

use super::{REMOTE, Remote, file_name, not_found, remote_path};
use crate::fs::vfs::{Capabilities, Vfs, VfsEntry};
use crate::lock::LockExt;
#[cfg(test)]
pub(in crate::fs::remote) use session::TEST_ROOT;
use session::{Config, Conn, Download, UploadStream, io_err};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use suppaftp::FtpResult;
use suppaftp::list::File as ListedFile;
use url::Url;

pub struct FtpVfs {
    config: Config,
    /// The connection listings and commands go through (reopened after a
    /// failure).
    control: Mutex<Option<Conn>>,
    /// Whether the server answers `MLST` (a [`Mlst`]).
    mlst: AtomicU8,
}

/// What is known about the server's `MLST`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Mlst {
    Untried = 0,
    Answers = 1,
    /// Not implemented: never asked again.
    Missing = 2,
}

impl std::fmt::Debug for FtpVfs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FtpVfs")
            .field("host", &self.config.host())
            .finish()
    }
}

impl FtpVfs {
    /// Connects to the server of `url` (`ftp://` or `ftps://`, explicit TLS).
    pub fn connect(url: &Url, secure: bool) -> io::Result<Remote> {
        let vfs = Self::open(url, secure)?;
        let start = PathBuf::from(remote_path(Path::new(url.path())));
        Ok(Remote {
            vfs: Arc::new(vfs),
            start,
        })
    }

    /// The adapter, logged in to the server of `url`.
    pub(in crate::fs::remote) fn open(url: &Url, secure: bool) -> io::Result<Self> {
        let config = Config::from_url(url, secure)?;
        let conn = config.open()?;
        Ok(Self {
            config,
            control: Mutex::new(Some(conn)),
            mlst: AtomicU8::new(Mlst::Untried as u8),
        })
    }

    /// `true` once the server answered an `MLST` (tests).
    #[cfg(test)]
    pub(in crate::fs::remote) fn answers_mlst(&self) -> bool {
        self.mlst.load(Ordering::Relaxed) == Mlst::Answers as u8
    }

    /// The `MLST` facts of `full`, or `None` when the server has no `MLST`
    /// (remembered, so it is not asked again) or its reply cannot be read.
    fn mlst(&self, full: &str) -> io::Result<Option<ListedFile>> {
        use suppaftp::FtpError;
        if self.mlst.load(Ordering::Relaxed) == Mlst::Missing as u8 {
            return Ok(None);
        }
        let reply = self.with_conn(|conn| match conn.mlst(Some(full)) {
            Ok(line) => Ok(Some(line)),
            // 500 / 502: not implemented; 501 / 504: not for this path.
            Err(FtpError::UnexpectedResponse(r)) if matches!(r.status.code(), 500 | 502) => {
                self.mlst.store(Mlst::Missing as u8, Ordering::Relaxed);
                Ok(None)
            }
            Err(FtpError::UnexpectedResponse(r)) if matches!(r.status.code(), 501 | 504) => {
                Ok(None)
            }
            Err(FtpError::BadResponse) => Ok(None),
            Err(e) => Err(e),
        })?;
        let Some(line) = reply else {
            return Ok(None);
        };
        self.mlst.store(Mlst::Answers as u8, Ordering::Relaxed);
        Ok(ListedFile::try_from(line.as_str()).ok())
    }

    /// Runs `op` on the control connection; a broken connection is opened
    /// again and `op` retried once.
    fn with_conn<T>(&self, mut op: impl FnMut(&mut Conn) -> FtpResult<T>) -> io::Result<T> {
        let mut control = self.control.lock_safe();
        for attempt in 0..2 {
            if control.is_none() {
                *control = Some(self.config.open()?);
            }
            let conn = control.as_mut().expect("connection opened above");
            match op(conn) {
                Ok(value) => return Ok(value),
                Err(suppaftp::FtpError::ConnectionError(e)) if attempt == 0 => {
                    tracing::debug!("ftp: reconnecting after {e}");
                    *control = None;
                }
                Err(e) => return Err(io_err(e)),
            }
        }
        Err(io::Error::from(io::ErrorKind::NotConnected))
    }

    /// The raw listing lines of `dir` (`MLSD`, else `LIST`).
    fn listing(&self, dir: &str) -> io::Result<Vec<String>> {
        self.with_conn(|conn| match conn.mlsd(Some(dir)) {
            Err(suppaftp::FtpError::UnexpectedResponse(r)) if r.status.code() >= 500 => {
                conn.list(Some(dir))
            }
            other => other,
        })
    }

    fn entry(dir: &Path, file: &ListedFile) -> Option<VfsEntry> {
        let name = file_name(file.name());
        if name.is_empty() || name == "." || name == ".." {
            return None;
        }
        let is_dir = file.is_directory();
        Some(VfsEntry {
            hidden: name.starts_with('.'),
            path: dir.join(&name),
            name,
            is_dir,
            is_symlink: file.is_symlink(),
            size: if is_dir { 0 } else { file.size() as u64 },
            modified: Some(file.modified()),
            ..VfsEntry::default()
        })
    }
}

impl Vfs for FtpVfs {
    fn capabilities(&self) -> Capabilities {
        REMOTE
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
        Ok(self
            .listing(&remote_path(dir))?
            .iter()
            .filter_map(|line| ListedFile::try_from(line.as_str()).ok())
            .filter_map(|file| Self::entry(dir, &file))
            .collect())
    }

    /// `MLST` where the server has it (one reply about the entry), else
    /// from the listing of the parent folder (every server has one).
    fn stat(&self, path: &Path) -> io::Result<VfsEntry> {
        let full = remote_path(path);
        let Some(parent) = path.parent().filter(|_| full != "/") else {
            return Ok(VfsEntry::dir(String::new(), path.to_path_buf()));
        };
        if let Some(entry) = self.mlst(&full)? {
            return Self::entry(parent, &entry)
                .map(|e| VfsEntry {
                    path: path.to_path_buf(),
                    ..e
                })
                .ok_or_else(|| not_found(path));
        }
        let name = file_name(&full);
        self.list(parent)?
            .into_iter()
            .find(|e| e.name == name)
            .map(|e| VfsEntry {
                path: path.to_path_buf(),
                ..e
            })
            .ok_or_else(|| not_found(path))
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn Read + Send>> {
        self.open_read_at(path, 0)
    }

    fn open_read_at(&self, path: &Path, offset: u64) -> io::Result<Box<dyn Read + Send>> {
        let download = Download::open(&self.config, &remote_path(path), offset)?;
        Ok(Box::new(download))
    }

    fn create_file(&self, path: &Path) -> io::Result<Box<dyn Write + Send>> {
        let upload = UploadStream::open(&self.config, &remote_path(path), false)?;
        Ok(Box::new(upload))
    }

    fn append_file(&self, path: &Path) -> io::Result<Box<dyn Write + Send>> {
        let upload = UploadStream::open(&self.config, &remote_path(path), true)?;
        Ok(Box::new(upload))
    }

    /// Servers differ on renaming over a file: see
    /// [`crate::fs::vfs::replace_by_moving_aside`].
    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        crate::fs::vfs::replace_by_moving_aside(self, from, to)
    }

    /// `MFMT` for the time, `SITE CHMOD` for the mode.
    fn set_metadata(
        &self,
        path: &Path,
        modified: Option<SystemTime>,
        mode: Option<u32>,
    ) -> io::Result<()> {
        use suppaftp::Status;
        let full = remote_path(path);
        if let Some(time) = modified {
            let stamp = chrono::DateTime::<chrono::Utc>::from(time).format("%Y%m%d%H%M%S");
            self.with_conn(|conn| {
                conn.custom_command(format!("MFMT {stamp} {full}"), &[Status::File])
            })?;
        }
        if let Some(mode) = mode {
            self.with_conn(|conn| conn.site(format!("CHMOD {:o} {full}", mode & 0o7777)))?;
        }
        Ok(())
    }

    fn mkdir(&self, path: &Path) -> io::Result<()> {
        self.with_conn(|conn| conn.mkdir(remote_path(path)))
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.with_conn(|conn| conn.rm(remote_path(path)))
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.with_conn(|conn| conn.rmdir(remote_path(path)))
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.with_conn(|conn| conn.rename(remote_path(from), remote_path(to)))
    }
}
