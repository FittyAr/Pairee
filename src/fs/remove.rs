//! The one deletion service of the filesystem layer.
//!
//! * [`remove_entry`] deletes one local entry — a file, a link (Windows
//!   folder links and junctions included, which need `remove_dir`) or an
//!   empty folder — retrying once after clearing a read-only attribute.
//!   Links are removed, never followed.
//! * [`remove_tree`] deletes a whole tree on any [`Vfs`], children before
//!   parents, through the shared [`Walker`] (links are removed, not
//!   entered). It is the default of [`Vfs::remove_all`].

use crate::fs::vfs::Vfs;
use crate::fs::walk::{LinkPolicy, WalkOptions, Walker};
use std::fs::Metadata;
use std::io;
use std::path::Path;

/// Deletes the local entry at `path` (file, link or empty folder).
pub fn remove_entry(path: &Path) -> io::Result<()> {
    let meta = std::fs::symlink_metadata(path)?;
    let remove = |p: &Path| {
        if removed_as_dir(&meta) {
            std::fs::remove_dir(p)
        } else {
            std::fs::remove_file(p)
        }
    };
    remove(path).or_else(|first| {
        if first.kind() != io::ErrorKind::PermissionDenied || make_writable(path, &meta).is_err() {
            return Err(first);
        }
        remove(path)
    })
}

/// Folders, and links that Windows stores as folders (directory symbolic
/// links and junctions), are deleted with `remove_dir`.
fn removed_as_dir(meta: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        meta.is_dir() || meta.file_type().is_symlink_dir()
    }
    #[cfg(not(windows))]
    {
        meta.is_dir()
    }
}

/// Clears the read-only attribute (Windows) or adds the owner's write bit
/// (Unix) of `path`; links are left alone.
fn make_writable(path: &Path, meta: &Metadata) -> io::Result<()> {
    if meta.file_type().is_symlink() {
        return Err(io::Error::from(io::ErrorKind::PermissionDenied));
    }
    let mut perms = meta.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let extra = if meta.is_dir() { 0o700 } else { 0o600 };
        perms.set_mode(perms.mode() | extra);
    }
    #[cfg(not(unix))]
    {
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
    }
    std::fs::set_permissions(path, perms)
}

/// Deletes `path` and everything below it on `vfs`. Links are removed,
/// never followed; a missing `path` is not an error. The first entry that
/// cannot be deleted stops the removal with its error.
pub fn remove_tree<V: Vfs + ?Sized>(vfs: &V, path: &Path) -> io::Result<()> {
    match vfs.stat(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
        Ok(entry) if !entry.is_real_dir() => return vfs.remove_file(path),
        Ok(_) => {}
    }
    let mut dirs = vec![path.to_path_buf()];
    for item in Walker::new(vfs, path, WalkOptions::with_links(LinkPolicy::Keep)) {
        let item = item?;
        if item.descends {
            dirs.push(item.entry.path);
        } else {
            vfs.remove_file(&item.entry.path)?;
        }
    }
    // Parents were found before their children: delete in reverse.
    dirs.iter().rev().try_for_each(|dir| vfs.remove_dir(dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::vfs::LocalVfs;

    #[test]
    fn removes_read_only_files_and_whole_trees() {
        let dir = tempfile::tempdir().unwrap();
        let tree = dir.path().join("tree");
        std::fs::create_dir_all(tree.join("sub")).unwrap();
        let locked = tree.join("sub").join("locked.txt");
        std::fs::write(&locked, b"x").unwrap();
        let mut perms = std::fs::metadata(&locked).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&locked, perms).unwrap();
        remove_entry(&locked).unwrap();
        assert!(!locked.exists());

        std::fs::write(tree.join("a.txt"), b"a").unwrap();
        remove_tree(&LocalVfs, &tree).unwrap();
        assert!(!tree.exists());
        remove_tree(&LocalVfs, &tree).unwrap();
    }

    /// A link to a folder inside the tree is removed, never followed.
    #[test]
    fn folder_links_are_removed_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(outside.join("keep.txt"), b"keep").unwrap();
        let tree = dir.path().join("tree");
        std::fs::create_dir(&tree).unwrap();
        let link = tree.join("link");
        if !crate::fs::walk::tests::symlink_dir(&outside, &link) {
            return;
        }
        remove_entry(&link).unwrap();
        assert!(std::fs::symlink_metadata(&link).is_err());
        assert!(crate::fs::walk::tests::symlink_dir(&outside, &link));
        remove_tree(&LocalVfs, &tree).unwrap();
        assert!(!tree.exists());
        assert_eq!(std::fs::read(outside.join("keep.txt")).unwrap(), b"keep");
    }
}
