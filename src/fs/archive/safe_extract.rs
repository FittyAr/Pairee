//! Safe-write guard shared by all archive extractors.
//!
//! Every extractor routes its writes through [`ExtractGuard`], which:
//! * rejects entry names with `..`, root or drive components (Zip-Slip);
//! * refuses to create anything beneath a pre-existing symlink/junction in
//!   the destination, so an archive can never write through a link;
//! * never overwrites an existing file or link — such entries are skipped
//!   and reported via [`ExtractGuard::skipped`];
//! * caps the number of entries and the total uncompressed bytes written,
//!   aborting decompression bombs early.

use anyhow::{Result, anyhow, bail};
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Component, Path, PathBuf};
use tokio::sync::mpsc;

use crate::fs::progress::ProgressUpdate;

/// Maximum total uncompressed bytes a single extraction may write.
pub const MAX_EXTRACT_TOTAL_BYTES: u64 = 32 * 1024 * 1024 * 1024;

/// Maximum number of entries a single extraction may process.
pub const MAX_EXTRACT_ENTRIES: usize = 500_000;

/// Size of the copy buffer used while enforcing the byte budget.
const COPY_CHUNK: usize = 64 * 1024;

pub struct ExtractGuard {
    root: PathBuf,
    max_bytes: u64,
    max_entries: usize,
    bytes: u64,
    entries: usize,
    skipped: Vec<PathBuf>,
}

impl ExtractGuard {
    /// Creates the destination root (if needed) with the default limits.
    pub fn new(root: &Path) -> Result<Self> {
        Self::with_limits(root, MAX_EXTRACT_TOTAL_BYTES, MAX_EXTRACT_ENTRIES)
    }

    pub fn with_limits(root: &Path, max_bytes: u64, max_entries: usize) -> Result<Self> {
        fs::create_dir_all(root)?;
        if fs::symlink_metadata(root)?.file_type().is_symlink() {
            // The user picked a link as destination; resolve it once so the
            // per-component checks below apply to the real directory.
            let resolved = fs::canonicalize(root)?;
            return Self::with_limits(&resolved, max_bytes, max_entries);
        }
        Ok(Self {
            root: root.to_path_buf(),
            max_bytes,
            max_entries,
            bytes: 0,
            entries: 0,
            skipped: Vec::new(),
        })
    }

    /// Entries that were skipped because something already existed there.
    #[cfg(test)]
    pub fn skipped(&self) -> &[PathBuf] {
        &self.skipped
    }

    /// Emits one `skipped` progress update per entry that was not written.
    pub fn report_skipped(&self, tx: &mpsc::Sender<ProgressUpdate>) {
        for rel in &self.skipped {
            let _ = tx.blocking_send(ProgressUpdate {
                current_file: rel.to_string_lossy().into_owned(),
                files_copied: 0,
                total_files: 0,
                bytes_copied: 0,
                total_bytes: 0,
                error: None,
                skipped: true,
            });
        }
    }

    /// Converts an untrusted entry name into a relative path made only of
    /// normal components. Both `/` and `\` are treated as separators.
    pub fn sanitize(name: &str) -> Result<PathBuf> {
        let normalized = name.replace('\\', "/");
        let mut rel = PathBuf::new();
        for component in Path::new(&normalized).components() {
            match component {
                Component::Normal(part) => rel.push(part),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    bail!("Refusing to extract entry with unsafe path: {name}");
                }
            }
        }
        Ok(rel)
    }

    /// Counts one archive entry against the entry limit.
    pub fn count_entry(&mut self) -> Result<()> {
        self.entries += 1;
        if self.entries > self.max_entries {
            bail!(
                "Archive has more than {} entries; refusing to extract (possible decompression bomb)",
                self.max_entries
            );
        }
        Ok(())
    }

    /// Fails early when an entry's declared size alone would blow the budget.
    pub fn check_declared_size(&self, size: u64) -> Result<()> {
        if self.bytes.saturating_add(size) > self.max_bytes {
            return Err(self.size_error());
        }
        Ok(())
    }

    fn size_error(&self) -> anyhow::Error {
        anyhow!(
            "Archive expands beyond {} bytes; refusing to extract (possible decompression bomb)",
            self.max_bytes
        )
    }

    /// Creates `rel` (and its parents) as real directories under the root.
    pub fn create_dir(&self, rel: &Path) -> Result<PathBuf> {
        let mut current = self.root.clone();
        for part in rel.components() {
            current.push(part);
            match fs::symlink_metadata(&current) {
                Ok(meta) if meta.file_type().is_symlink() => {
                    bail!(
                        "Refusing to extract through existing link: {}",
                        current.display()
                    );
                }
                Ok(meta) if meta.is_dir() => {}
                Ok(_) => bail!(
                    "Cannot create directory, a file is in the way: {}",
                    current.display()
                ),
                Err(e) if e.kind() == ErrorKind::NotFound => match fs::create_dir(&current) {
                    Ok(()) => {}
                    Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                        // Lost a race: re-validate on the next pass.
                        if fs::symlink_metadata(&current)?.file_type().is_symlink() {
                            bail!(
                                "Refusing to extract through existing link: {}",
                                current.display()
                            );
                        }
                    }
                    Err(e) => return Err(e.into()),
                },
                Err(e) => return Err(e.into()),
            }
        }
        Ok(current)
    }

    /// Returns the destination for a new leaf entry after creating its
    /// parents, or `None` (recorded as skipped) if something already exists.
    fn prepare_leaf(&mut self, rel: &Path) -> Result<Option<PathBuf>> {
        let file_name = rel
            .file_name()
            .ok_or_else(|| anyhow!("Archive entry has an empty name"))?;
        let parent = self.create_dir(rel.parent().unwrap_or(Path::new("")))?;
        let target = parent.join(file_name);
        match fs::symlink_metadata(&target) {
            Ok(_) => {
                tracing::warn!("extract: skipping existing {}", target.display());
                self.skipped.push(rel.to_path_buf());
                Ok(None)
            }
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Some(target)),
            Err(e) => Err(e.into()),
        }
    }

    /// Creates a brand-new file for `rel`. Returns `None` if the path already
    /// exists (the entry is skipped, never overwritten).
    pub fn create_file(&mut self, rel: &Path) -> Result<Option<File>> {
        let Some(target) = self.prepare_leaf(rel)? else {
            return Ok(None);
        };
        // `create_new` is O_CREAT|O_EXCL: it fails instead of following a
        // link planted between the check above and this open.
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
        {
            Ok(f) => Ok(Some(f)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                self.skipped.push(rel.to_path_buf());
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Creates a symlink entry. Links are never written through by this
    /// guard, so a hostile target cannot be used to escape the destination.
    #[cfg(unix)]
    pub fn create_symlink(&mut self, rel: &Path, link_target: &Path) -> Result<()> {
        if let Some(target) = self.prepare_leaf(rel)? {
            std::os::unix::fs::symlink(link_target, target)?;
        }
        Ok(())
    }

    /// Validates an entry that an external tool will write, without
    /// creating anything: counts it, adds its declared size to the budget,
    /// rejects existing links on its path and records it as skipped if a
    /// non-directory already exists at the target.
    pub fn preflight(&mut self, rel: &Path, declared_size: u64, is_dir: bool) -> Result<()> {
        self.count_entry()?;
        self.check_declared_size(declared_size)?;
        self.bytes += declared_size;

        let mut current = self.root.clone();
        for part in rel.components() {
            current.push(part);
            match fs::symlink_metadata(&current) {
                Ok(meta) if meta.file_type().is_symlink() => {
                    bail!(
                        "Refusing to extract through existing link: {}",
                        current.display()
                    );
                }
                Ok(meta) if current.as_path() == self.root.join(rel) => {
                    if !(is_dir && meta.is_dir()) {
                        self.skipped.push(rel.to_path_buf());
                    }
                }
                Ok(meta) if !meta.is_dir() => bail!(
                    "Cannot create directory, a file is in the way: {}",
                    current.display()
                ),
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::NotFound => break,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    /// Copies `reader` into `out`, enforcing the total byte budget.
    pub fn copy_limited(&mut self, reader: &mut dyn Read, out: &mut File) -> Result<u64> {
        let mut buf = vec![0u8; COPY_CHUNK];
        let mut written = 0u64;
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            };
            self.bytes += n as u64;
            if self.bytes > self.max_bytes {
                return Err(self.size_error());
            }
            out.write_all(&buf[..n])?;
            written += n as u64;
        }
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_rejects_traversal() {
        assert!(ExtractGuard::sanitize("../evil").is_err());
        assert!(ExtractGuard::sanitize("a/../../evil").is_err());
        assert!(ExtractGuard::sanitize("/etc/passwd").is_err());
        assert!(ExtractGuard::sanitize("..\\evil").is_err());
        assert_eq!(
            ExtractGuard::sanitize("./a/b.txt").unwrap(),
            Path::new("a").join("b.txt")
        );
    }

    #[test]
    fn existing_file_is_skipped_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("keep.txt"), b"original").unwrap();
        let mut guard = ExtractGuard::new(dir.path()).unwrap();
        assert!(guard.create_file(Path::new("keep.txt")).unwrap().is_none());
        assert_eq!(guard.skipped(), &[PathBuf::from("keep.txt")]);
        assert_eq!(fs::read(dir.path().join("keep.txt")).unwrap(), b"original");
    }

    #[test]
    fn byte_and_entry_limits_are_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let mut guard = ExtractGuard::with_limits(dir.path(), 10, 2).unwrap();
        guard.count_entry().unwrap();
        guard.count_entry().unwrap();
        assert!(guard.count_entry().is_err());
        assert!(guard.check_declared_size(11).is_err());

        let mut out = guard.create_file(Path::new("big")).unwrap().unwrap();
        let mut src: &[u8] = &[0u8; 64];
        assert!(guard.copy_limited(&mut src, &mut out).is_err());
    }

    #[test]
    fn never_writes_through_existing_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(outside.path(), dir.path().join("link"));
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_dir(outside.path(), dir.path().join("link"));
        if made.is_err() {
            // Symlink creation needs Developer Mode / admin on Windows.
            return;
        }
        let mut guard = ExtractGuard::new(dir.path()).unwrap();
        assert!(guard.create_file(Path::new("link/pwn.txt")).is_err());
        assert!(guard.create_dir(Path::new("link/sub")).is_err());
        assert!(!outside.path().join("pwn.txt").exists());
        assert!(!outside.path().join("sub").exists());
    }

    #[cfg(unix)]
    #[test]
    fn never_overwrites_through_dangling_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path().join("f"), dir.path().join("f")).unwrap();
        let mut guard = ExtractGuard::new(dir.path()).unwrap();
        assert!(guard.create_file(Path::new("f")).unwrap().is_none());
        assert!(!outside.path().join("f").exists());
    }
}
