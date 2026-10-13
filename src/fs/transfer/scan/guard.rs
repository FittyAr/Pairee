//! Guards that keep a transfer from destroying its own source: a file
//! copied onto itself, a folder copied into itself. Both compare filesystem
//! identities ([`Vfs::identity`]), so links, hard links, `.`/`..` and letter
//! case cannot fool them.

use crate::fs::vfs::Vfs;
use std::path::Path;

/// `true` when `inner` is `outer` or lies below it (`inner` itself need not
/// exist yet). `false` when the filesystem cannot identify `outer`.
pub fn is_within(fs: &dyn Vfs, inner: &Path, outer: &Path) -> bool {
    let Some(outer_id) = fs.identity(outer) else {
        return false;
    };
    inner
        .ancestors()
        .filter(|p| !p.as_os_str().is_empty())
        .any(|ancestor| fs.identity(ancestor).as_ref() == Some(&outer_id))
}

/// `true` when `src` and `dst`, on one filesystem, are the same file.
pub fn same_entry(fs: &dyn Vfs, src: &Path, dst: &Path) -> bool {
    match (fs.identity(src), fs.identity(dst)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::vfs::LocalVfs;

    #[test]
    fn detects_folders_inside_themselves_even_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        std::fs::create_dir_all(a.join("sub")).unwrap();
        assert!(is_within(&LocalVfs, &a, &a));
        assert!(is_within(&LocalVfs, &a.join("sub"), &a));
        assert!(is_within(
            &LocalVfs,
            &a.join("sub").join("new").join("a"),
            &a
        ));
        assert!(is_within(
            &LocalVfs,
            &a.join("sub").join("..").join("x"),
            &a
        ));
        assert!(!is_within(&LocalVfs, &dir.path().join("b"), &a));
        assert!(!is_within(&LocalVfs, dir.path(), &a));
    }

    #[test]
    fn same_entry_needs_two_existing_identical_files() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("f.txt");
        std::fs::write(&f, b"x").unwrap();
        assert!(same_entry(
            &LocalVfs,
            &f,
            &dir.path().join(".").join("f.txt")
        ));
        assert!(!same_entry(&LocalVfs, &f, &dir.path().join("g.txt")));
    }
}
