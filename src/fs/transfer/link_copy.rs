//! Copying a symbolic link as a link: the new link gets exactly the same
//! target text (a relative target stays relative, so a moved tree of
//! relative links keeps working). It is made under the temporary name and
//! committed like a file copy, so a failure leaves the destination as it
//! was, and a real folder at the destination is never replaced.

use super::part::{Replaced, commit, discard, part_path};
use std::io;
use std::path::Path;

/// Recreates the link `src` at `dst`; an existing file or link there is
/// replaced (trashed first when `trash_old`).
pub fn recreate_link(src: &Path, dst: &Path, trash_old: bool) -> io::Result<Replaced> {
    let target = std::fs::read_link(src)?;
    let part = part_path(dst);
    discard(&part);
    make_link(src, &target, &part)?;
    commit(&part, dst, trash_old).inspect_err(|_| discard(&part))
}

/// Creates a link at `link` with the target text `target`, of the same
/// kind (file or folder link, on Windows) as the link `like`.
#[cfg(unix)]
fn make_link(_like: &Path, target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn make_link(like: &Path, target: &Path, link: &Path) -> io::Result<()> {
    use std::os::windows::fs::FileTypeExt;
    if std::fs::symlink_metadata(like)?
        .file_type()
        .is_symlink_dir()
    {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    }
}

#[cfg(not(any(unix, windows)))]
fn make_link(_like: &Path, _target: &Path, _link: &Path) -> io::Result<()> {
    Err(io::ErrorKind::Unsupported.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_link(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(target, link);
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(target, link);
        made.is_ok()
    }

    #[test]
    fn relative_targets_stay_relative() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();
        std::fs::write(src_dir.join("data.txt"), b"x").unwrap();
        if !file_link(Path::new("data.txt"), &src_dir.join("link")) {
            return;
        }
        recreate_link(&src_dir.join("link"), &dst_dir.join("link"), false).unwrap();
        assert_eq!(
            std::fs::read_link(dst_dir.join("link")).unwrap(),
            Path::new("data.txt")
        );
    }

    #[test]
    fn a_real_folder_is_never_replaced_by_a_link() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("t.txt"), b"x").unwrap();
        let link = dir.path().join("link");
        if !file_link(&dir.path().join("t.txt"), &link) {
            return;
        }
        let folder = dir.path().join("folder");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("keep.txt"), b"keep").unwrap();
        assert!(recreate_link(&link, &folder, false).is_err());
        assert_eq!(std::fs::read(folder.join("keep.txt")).unwrap(), b"keep");
        assert!(!part_path(&folder).exists());
    }
}
