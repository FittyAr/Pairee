//! Safe-write guard shared by all archive extractors.
//!
//! Every extractor routes its writes through [`ExtractGuard`], which:
//! * rejects entry names with `..`, root or drive components (Zip-Slip);
//! * rejects names the local filesystem cannot store as they are (on
//!   Windows: `:` alternate data streams, `CON` / `NUL` / `COM1`...
//!   devices with or without an extension, trailing dots and spaces);
//! * refuses to create anything beneath a pre-existing symlink/junction in
//!   the destination, so an archive can never write through a link;
//! * never overwrites an existing file or link — such entries are skipped
//!   and reported via [`ExtractGuard::skipped`] — except one this very
//!   extraction wrote: a later entry of the same name replaces it, as tar
//!   archives updated with `tar -u` expect, and the regular files the user
//!   chose to replace ([`ExtractGuard::replace_existing`]);
//! * caps the number of entries and the total uncompressed bytes written,
//!   aborting decompression bombs early.

use crate::config::localization::{t, t_fill};
use anyhow::{Result, anyhow, bail};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Component, Path, PathBuf};

use crate::fs::multi_rename::TargetFs;
use crate::fs::progress::ArchiveReport;

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
    /// Files and links this extraction created (a later entry of the same
    /// name replaces them).
    written: HashSet<PathBuf>,
    /// Entries (relative paths) whose existing regular file is replaced.
    replace: HashSet<PathBuf>,
    /// Replaced files, renamed aside until their new copy is complete.
    aside: std::collections::HashMap<PathBuf, PathBuf>,
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
            written: HashSet::new(),
            replace: HashSet::new(),
            aside: std::collections::HashMap::new(),
        })
    }

    /// The entries `rels` replace a regular file already at their path
    /// (the user's answer to the conflict); links and folders in the way
    /// still make them skipped.
    pub fn replace_existing(&mut self, rels: impl IntoIterator<Item = PathBuf>) {
        self.replace.extend(rels);
    }

    /// Entries that were skipped because something already existed there.
    #[cfg(test)]
    pub fn skipped(&self) -> &[PathBuf] {
        &self.skipped
    }

    /// Reports every entry that was not written (something existed there).
    pub fn report_skipped(&self, report: &mut ArchiveReport) {
        for rel in &self.skipped {
            report.skipped(rel);
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
                    bail!(t_fill("archive_unsafe_entry_path", &[name]));
                }
            }
        }
        Ok(rel)
    }

    /// Counts one archive entry against the entry limit.
    pub fn count_entry(&mut self) -> Result<()> {
        self.entries += 1;
        if self.entries > self.max_entries {
            bail!(t_fill(
                "archive_too_many_entries",
                &[&self.max_entries.to_string()]
            ));
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
        anyhow!(t_fill(
            "archive_expands_too_much",
            &[&bytesize::ByteSize::b(self.max_bytes).to_string()]
        ))
    }

    /// Creates `rel` (and its parents) as real directories under the root.
    pub fn create_dir(&self, rel: &Path) -> Result<PathBuf> {
        let mut current = self.root.clone();
        for part in rel.components() {
            ensure_native_name(part)?;
            current.push(part);
            match fs::symlink_metadata(&current) {
                Ok(meta) if meta.file_type().is_symlink() => bail!(through_link(&current)),
                Ok(meta) if meta.is_dir() => {}
                Ok(_) => bail!(file_in_the_way(&current)),
                Err(e) if e.kind() == ErrorKind::NotFound => match fs::create_dir(&current) {
                    Ok(()) => {}
                    Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                        // Lost a race: re-validate on the next pass.
                        if fs::symlink_metadata(&current)?.file_type().is_symlink() {
                            bail!(through_link(&current));
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
            .ok_or_else(|| anyhow!(t("archive_entry_empty_name")))?;
        ensure_native_name(Component::Normal(file_name))?;
        let parent = self.create_dir(rel.parent().unwrap_or(Path::new("")))?;
        let target = parent.join(file_name);
        match fs::symlink_metadata(&target) {
            Ok(meta) if !meta.is_dir() && self.written.contains(&target) => {
                // An older copy of this entry, written by this extraction.
                fs::remove_file(&target)?;
                Ok(Some(target))
            }
            Ok(meta) if meta.is_file() && self.replace.contains(rel) => {
                // Kept aside until the new file is complete: a failed or
                // cancelled write puts the old one back.
                let aside = set_aside(&target, &meta)?;
                self.aside.insert(target.clone(), aside);
                Ok(Some(target))
            }
            Ok(_) => {
                tracing::warn!("extract: skipping existing {}", target.display());
                self.skipped.push(rel.to_path_buf());
                Ok(None)
            }
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Some(target)),
            Err(e) => Err(e.into()),
        }
    }

    /// Creates a brand-new file for `rel` and returns it with its path, or
    /// `None` if the path already exists (the entry is skipped, never
    /// overwritten).
    fn create_file(&mut self, rel: &Path) -> Result<Option<(File, PathBuf)>> {
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
            Ok(f) => {
                self.written.insert(target.clone());
                Ok(Some((f, target)))
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                self.restore_aside(&target);
                self.skipped.push(rel.to_path_buf());
                Ok(None)
            }
            Err(e) => {
                self.restore_aside(&target);
                Err(e.into())
            }
        }
    }

    /// The file replaced at `target` is no longer needed.
    fn drop_aside(&mut self, target: &Path) {
        if let Some(aside) = self.aside.remove(target) {
            let _ = fs::remove_file(aside);
        }
    }

    /// Puts the file replaced at `target` back (its new copy failed).
    fn restore_aside(&mut self, target: &Path) {
        if let Some(aside) = self.aside.remove(target) {
            let _ = fs::rename(aside, target);
        }
    }

    /// Writes the file entry `rel` from `data` (skipped, returning `None`,
    /// when something already exists there). A failure or a cancellation
    /// (`cancelled` is polled between chunks) removes the partial file, so
    /// a later extraction does not skip it as already present.
    pub fn write_file(
        &mut self,
        rel: &Path,
        data: &mut dyn Read,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<File>> {
        let Some((mut out, target)) = self.create_file(rel)? else {
            return Ok(None);
        };
        match self.copy_limited(data, &mut out, cancelled) {
            Ok(_) => {
                self.drop_aside(&target);
                Ok(Some(out))
            }
            Err(e) => {
                drop(out);
                let _ = fs::remove_file(&target);
                self.written.remove(&target);
                self.restore_aside(&target);
                Err(e)
            }
        }
    }

    /// Creates a symlink entry. Links are never written through by this
    /// guard, so a hostile target cannot be used to escape the destination.
    #[cfg(unix)]
    pub fn create_symlink(&mut self, rel: &Path, link_target: &Path) -> Result<()> {
        if let Some(target) = self.prepare_leaf(rel)? {
            if let Err(e) = std::os::unix::fs::symlink(link_target, &target) {
                self.restore_aside(&target);
                return Err(e.into());
            }
            self.drop_aside(&target);
            self.written.insert(target);
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
            ensure_native_name(part)?;
            current.push(part);
            match fs::symlink_metadata(&current) {
                Ok(meta) if meta.file_type().is_symlink() => bail!(through_link(&current)),
                Ok(meta) if current.as_path() == self.root.join(rel) => {
                    if !(is_dir && meta.is_dir()) {
                        self.skipped.push(rel.to_path_buf());
                    }
                }
                Ok(meta) if !meta.is_dir() => bail!(file_in_the_way(&current)),
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::NotFound => break,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    /// Copies `reader` into `out`, enforcing the total byte budget.
    fn copy_limited(
        &mut self,
        reader: &mut dyn Read,
        out: &mut File,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<u64> {
        let mut buf = vec![0u8; COPY_CHUNK];
        let mut written = 0u64;
        loop {
            if cancelled() {
                return Err(crate::fs::error::cancelled());
            }
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

/// Renames the existing file `target` to a hidden name next to it (made
/// writable first: Windows cannot rename over or delete a read-only file
/// later).
fn set_aside(target: &Path, meta: &fs::Metadata) -> Result<PathBuf> {
    if meta.permissions().readonly() {
        let mut writable = meta.permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        writable.set_readonly(false);
        fs::set_permissions(target, writable)?;
    }
    let name = crate::fs::file_name_lossy(target);
    let aside = target.with_file_name(format!(".{name}.{}.pairee-old", uuid::Uuid::new_v4()));
    fs::rename(target, &aside)?;
    Ok(aside)
}

/// `Err` when the local filesystem cannot store the entry name `part` as it
/// is. [`ExtractGuard::sanitize`] stays platform-neutral (browsing an
/// archive lists every entry); the rules of the machine apply when
/// something is about to be written.
fn ensure_native_name(part: Component<'_>) -> Result<()> {
    let name = part.as_os_str().to_string_lossy();
    if TargetFs::local().is_valid_name(&name) {
        Ok(())
    } else {
        bail!(t_fill("archive_unsafe_entry_name", &[&name]))
    }
}

/// An extraction would write through the existing link `path`.
fn through_link(path: &Path) -> String {
    t_fill(
        "archive_extract_through_link",
        &[&path.display().to_string()],
    )
}

/// A folder entry would replace the existing file `path`.
fn file_in_the_way(path: &Path) -> String {
    t_fill("archive_file_in_the_way", &[&path.display().to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Names Windows would turn into streams or devices are refused where
    /// they are written (other systems store them as they are).
    #[test]
    fn names_the_system_cannot_store_are_refused_when_writing() {
        let dir = tempfile::tempdir().unwrap();
        let mut guard = ExtractGuard::new(dir.path()).unwrap();
        for name in ["d/a:stream", "con", "NUL.txt", "dir./x", "trail "] {
            let rel = ExtractGuard::sanitize(name).unwrap();
            let refused = guard.preflight(&rel, 1, false).is_err()
                || guard.write_file(&rel, &mut &b"x"[..], &|| false).is_err();
            assert_eq!(refused, cfg!(windows), "{name}");
        }
        // A plain name always goes through.
        let ok = Path::new("fine.txt");
        assert!(guard.write_file(ok, &mut &b"x"[..], &|| false).is_ok());
    }

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
        let mut data: &[u8] = b"new";
        let written = guard.write_file(Path::new("keep.txt"), &mut data, &|| false);
        assert!(written.unwrap().is_none());
        assert_eq!(guard.skipped(), &[PathBuf::from("keep.txt")]);
        assert_eq!(fs::read(dir.path().join("keep.txt")).unwrap(), b"original");
    }

    /// `tar -u` appends newer versions: the last one is what extracts, but
    /// a file that was there before the extraction is still never touched.
    #[test]
    fn a_later_entry_replaces_only_what_this_extraction_wrote() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("keep.txt"), b"original").unwrap();
        let mut guard = ExtractGuard::new(dir.path()).unwrap();
        for data in [&b"old"[..], &b"new"[..]] {
            let mut data = data;
            guard
                .write_file(Path::new("a.txt"), &mut data, &|| false)
                .unwrap();
        }
        assert_eq!(fs::read(dir.path().join("a.txt")).unwrap(), b"new");
        let mut data: &[u8] = b"x";
        let kept = guard.write_file(Path::new("keep.txt"), &mut data, &|| false);
        assert!(kept.unwrap().is_none());
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

        let mut src: &[u8] = &[0u8; 64];
        assert!(
            guard
                .write_file(Path::new("big"), &mut src, &|| false)
                .is_err()
        );
        // The partial file is removed: a new extraction writes it again
        // instead of skipping it as already present.
        assert!(!dir.path().join("big").exists());
    }

    #[test]
    fn cancelled_write_leaves_no_partial_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut guard = ExtractGuard::new(dir.path()).unwrap();
        let mut src: &[u8] = &[1u8; 16];
        assert!(
            guard
                .write_file(Path::new("f"), &mut src, &|| true)
                .is_err()
        );
        assert!(!dir.path().join("f").exists());
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
        let mut data: &[u8] = b"x";
        assert!(
            guard
                .write_file(Path::new("link/pwn.txt"), &mut data, &|| false)
                .is_err()
        );
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
        let mut data: &[u8] = b"x";
        let written = guard.write_file(Path::new("f"), &mut data, &|| false);
        assert!(written.unwrap().is_none());
        assert!(!outside.path().join("f").exists());
    }
}
