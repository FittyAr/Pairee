//! In-place archive editing, shared by every writable format: an edit
//! ([`ArchiveEdit`]) is applied by rewriting the archive to a temporary
//! file next to it, which gets the archive's permissions and is then
//! atomically renamed over the original, so a failure (or a cancellation)
//! never leaves a half-written archive behind.

use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::safe_extract::ExtractGuard;

/// Content of an entry added to the archive.
#[derive(Debug, Clone)]
pub enum EntrySource {
    Dir,
    Bytes(Vec<u8>),
    /// A local file (or folder), streamed into the archive with its
    /// permissions and modification time.
    File(PathBuf),
}

/// Changes applied by one rewrite. Paths are relative to the archive root.
#[derive(Debug, Clone, Default)]
pub struct ArchiveEdit {
    /// Entries removed with everything below them.
    pub remove: Vec<PathBuf>,
    /// Entries added; an existing entry with the same path is replaced.
    pub add: Vec<(PathBuf, EntrySource)>,
}

/// Runs after each added entry (progress, cancellation: an error aborts
/// the rewrite and keeps the original archive).
pub type OnAdded<'a> = dyn FnMut(&Path) -> io::Result<()> + 'a;

impl ArchiveEdit {
    /// Which stored entries the rewrite leaves out.
    pub fn dropper(&self) -> Dropper<'_> {
        Dropper {
            replaced: self.add.iter().map(|(rel, _)| rel.as_path()).collect(),
            removed: &self.remove,
        }
    }
}

/// Decides which stored entries an edit drops (removed, or replaced by an
/// added entry of the same path).
pub struct Dropper<'a> {
    replaced: HashSet<&'a Path>,
    removed: &'a [PathBuf],
}

impl Dropper<'_> {
    /// `true` when the stored entry `name` is not copied to the new archive.
    /// Names that cannot be made safe are kept as they are.
    pub fn drops(&self, name: &str) -> bool {
        ExtractGuard::sanitize(name).is_ok_and(|rel| {
            self.replaced.contains(rel.as_path()) || self.removed.iter().any(|r| rel.starts_with(r))
        })
    }
}

/// `/`-separated entry name of `rel` (folders end with `/` when `dir_slash`).
pub fn entry_name(rel: &Path, dir_slash: bool) -> String {
    let mut name = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    if dir_slash {
        name.push('/');
    }
    name
}

/// Writes a new version of `archive` with `write` (which gets a temporary
/// file next to it) and moves it over the original, keeping the archive's
/// permissions. Whatever `write` opens (the old archive) is closed when it
/// returns: Windows cannot replace a file that is still open.
pub fn replace_archive(archive: &Path, write: impl FnOnce(fs::File) -> Result<()>) -> Result<()> {
    // Through a link, the archive it points to is rewritten (renaming over
    // the link would replace the link and leave the archive as it was).
    let real = fs::canonicalize(archive)?;
    let archive = real.as_path();
    let dir = archive.parent().unwrap_or(Path::new("."));
    let tmp = tempfile::NamedTempFile::new_in(dir)?;
    write(tmp.as_file().try_clone()?)?;
    tmp.as_file().sync_all()?;
    // The temporary file is private (0600): give it the archive's mode.
    tmp.as_file()
        .set_permissions(fs::metadata(archive)?.permissions())?;
    tmp.persist(archive)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_removed_subtrees_and_replaced_entries() {
        let edit = ArchiveEdit {
            remove: vec![PathBuf::from("old")],
            add: vec![(PathBuf::from("a.txt"), EntrySource::Dir)],
        };
        let dropper = edit.dropper();
        assert!(dropper.drops("old/x.txt"));
        assert!(dropper.drops("old/"));
        assert!(dropper.drops("./a.txt"));
        assert!(!dropper.drops("older.txt"));
        assert!(!dropper.drops("keep/a.txt"));
    }

    #[test]
    fn names_use_slashes() {
        let rel = Path::new("a").join("b");
        assert_eq!(entry_name(&rel, false), "a/b");
        assert_eq!(entry_name(&rel, true), "a/b/");
    }
}
