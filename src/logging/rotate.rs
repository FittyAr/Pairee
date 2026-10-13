//! Size-capped, append-only log file: `app.log` plus one older `app.log.1`.
//!
//! The file is opened in append mode, so several Pairee processes (a second
//! window, the elevated helper) add to the same log instead of truncating
//! each other's lines. It is rotated when it grows past the cap, at start
//! and while running; a rotation that fails (another process holds the
//! file on Windows) is retried at the next write past the cap.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Size at which `app.log` is moved to `app.log.1`.
pub const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

/// Path of the previous log: `app.log` → `app.log.1`.
pub fn rotated_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".1");
    PathBuf::from(name)
}

/// Append-only writer over the log file that rotates past `max_bytes`.
pub struct RotatingFile {
    path: PathBuf,
    /// `None` only while rotating, or after a reopen failed.
    file: Option<File>,
    written: u64,
    max_bytes: u64,
}

impl RotatingFile {
    /// Opens `path` for appending, rotating it first when it is already
    /// past `max_bytes`.
    pub fn open(path: &Path, max_bytes: u64) -> io::Result<Self> {
        if std::fs::metadata(path).is_ok_and(|m| m.len() >= max_bytes) {
            // Best effort: another process may hold the file (Windows).
            let _ = std::fs::rename(path, rotated_path(path));
        }
        let file = open_append(path)?;
        let written = file.metadata().map(|m| m.len()).unwrap_or(0);
        Ok(Self {
            path: path.to_path_buf(),
            file: Some(file),
            written,
            max_bytes,
        })
    }

    /// Moves the current file to `.1` and starts a new one. The handle is
    /// closed first, since Windows cannot rename a file this process holds.
    /// When another process holds it the rename fails and the next attempt
    /// comes after another `max_bytes`.
    fn rotate(&mut self) -> io::Result<()> {
        if let Some(mut file) = self.file.take() {
            let _ = file.flush();
        }
        let _ = std::fs::rename(&self.path, rotated_path(&self.path));
        self.file = Some(open_append(&self.path)?);
        self.written = 0;
        Ok(())
    }
}

impl Write for RotatingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.written >= self.max_bytes || self.file.is_none() {
            self.rotate()?;
        }
        let file = self.file.as_mut().ok_or(io::ErrorKind::NotFound)?;
        let n = file.write(buf)?;
        self.written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.as_mut().map_or(Ok(()), Write::flush)
    }
}

fn open_append(path: &Path) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_instead_of_truncating() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.log");
        std::fs::write(&path, b"first run\n").unwrap();
        let mut log = RotatingFile::open(&path, 1024).unwrap();
        log.write_all(b"second run\n").unwrap();
        drop(log);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "first run\nsecond run\n"
        );
    }

    #[test]
    fn rotates_an_oversized_log_at_start() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.log");
        std::fs::write(&path, vec![b'x'; 64]).unwrap();
        let mut log = RotatingFile::open(&path, 32).unwrap();
        log.write_all(b"new\n").unwrap();
        drop(log);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new\n");
        assert_eq!(std::fs::read(rotated_path(&path)).unwrap().len(), 64);
    }

    #[test]
    fn rotates_while_running_and_keeps_one_old_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.log");
        let mut log = RotatingFile::open(&path, 8).unwrap();
        log.write_all(b"aaaaaaaa").unwrap();
        log.write_all(b"bbbbbbbb").unwrap();
        log.write_all(b"cc").unwrap();
        drop(log);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "cc");
        assert_eq!(
            std::fs::read_to_string(rotated_path(&path)).unwrap(),
            "bbbbbbbb"
        );
    }

    #[test]
    fn two_writers_share_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.log");
        let mut a = RotatingFile::open(&path, 1024).unwrap();
        let mut b = RotatingFile::open(&path, 1024).unwrap();
        a.write_all(b"a\n").unwrap();
        b.write_all(b"b\n").unwrap();
        a.write_all(b"a\n").unwrap();
        drop((a, b));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "a\nb\na\n");
    }
}
