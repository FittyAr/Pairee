//! Atomic writes: a file is written to `<name>.pairee-part` next to its
//! destination, flushed to disk and only then renamed over the target, so a
//! cancelled or failed copy never leaves a truncated file behind and never
//! loses the file it was replacing.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};

/// Suffix of the temporary file a copy is written to.
pub const PART_SUFFIX: &str = ".pairee-part";

/// The temporary file `dst` is written to before it is committed.
///
/// A name that is already near the filesystem's limit (255 bytes) would not
/// fit the suffix: its part file gets the start of the name, a hash of the
/// whole name (so two long names never share a part, and a resumed copy
/// finds its own) and the suffix.
pub fn part_path(dst: &Path) -> PathBuf {
    let name = dst.file_name().unwrap_or_default();
    let mut part = name.to_os_string();
    part.push(PART_SUFFIX);
    if part.len() > NAME_MAX {
        part = shortened_part_name(name);
    }
    dst.with_file_name(part)
}

/// Longest file name (bytes) the filesystems Pairee runs on accept.
const NAME_MAX: usize = 255;

fn shortened_part_name(name: &OsStr) -> OsString {
    let hash = blake3::hash(name.as_encoded_bytes()).to_hex();
    let tag = format!(".{}{PART_SUFFIX}", &hash[..16]);
    let text = name.to_string_lossy();
    let mut end = (NAME_MAX - tag.len()).min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    OsString::from(format!("{}{tag}", &text[..end]))
}

/// Deletes a temporary file (or link) that will not be committed; a
/// missing one is fine.
pub fn discard(part: &Path) {
    let _ = crate::fs::remove::remove_entry(part);
}

/// What happened to an existing destination when a copy replaced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Replaced {
    /// Nothing was there.
    #[default]
    Nothing,
    /// The old file went to the trash (an undo can bring it back).
    Trashed,
    /// The old file was overwritten for good.
    Overwritten,
}

impl Replaced {
    /// Something existed at the destination.
    pub fn any(self) -> bool {
        self != Self::Nothing
    }
}

/// Renames the finished `part` over `dst`. An existing file at `dst` is
/// sent to the trash first when `trash_old` (falling back to overwriting it
/// when the trash cannot take it); a folder there is never replaced.
pub fn commit(part: &Path, dst: &Path, trash_old: bool) -> io::Result<Replaced> {
    let mut replaced = Replaced::Nothing;
    if let Ok(meta) = std::fs::symlink_metadata(dst) {
        if meta.is_dir() {
            return Err(folder_in_the_way(dst));
        }
        replaced = Replaced::Overwritten;
        if trash_old && trash::delete(dst).is_ok() {
            replaced = Replaced::Trashed;
        } else {
            allow_replacing(dst, &meta)?;
        }
    }
    std::fs::rename(part, dst)?;
    Ok(replaced)
}

/// The error of a copy whose destination is a folder: a file never
/// replaces one (some servers would delete the whole folder to do it).
pub fn folder_in_the_way(dst: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        crate::config::localization::t("transfer_folder_in_the_way").replacen(
            "{}",
            &dst.to_string_lossy(),
            1,
        ),
    )
}

/// Windows refuses to rename over a read-only file (clear the flag first)
/// or over a folder link (remove the link first: it only points elsewhere).
fn allow_replacing(dst: &Path, meta: &std::fs::Metadata) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        if meta.file_type().is_symlink_dir() {
            return crate::fs::remove::remove_entry(dst);
        }
        if meta.permissions().readonly() && !meta.file_type().is_symlink() {
            let mut perms = meta.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(dst, perms);
        }
    }
    #[cfg(not(windows))]
    let _ = (dst, meta);
    Ok(())
}

/// Writes `path` atomically with the primitives of one filesystem: `write`
/// fills the temporary file, `rename` moves it over `path` and `remove`
/// deletes it after a failure.
pub fn write_atomically(
    path: &Path,
    write: impl FnOnce(&Path) -> io::Result<()>,
    rename: impl FnOnce(&Path, &Path) -> io::Result<()>,
    remove: impl Fn(&Path),
) -> io::Result<()> {
    let part = part_path(path);
    let result = write(&part).and_then(|()| rename(&part, path));
    if result.is_err() {
        remove(&part);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_path_appends_the_suffix() {
        let dst = Path::new("dir").join("a.txt");
        assert_eq!(part_path(&dst), Path::new("dir").join("a.txt.pairee-part"));
    }

    #[test]
    fn long_names_get_a_shortened_part_name_that_fits() {
        let dir = Path::new("dir");
        // Two names that only differ at their very end, made of 2-byte chars.
        let long = |end: char| format!("{}{end}", "\u{e9}".repeat(127));
        let (a, b) = (dir.join(long('a')), dir.join(long('b')));
        let (pa, pb) = (part_path(&a), part_path(&b));
        for part in [&pa, &pb] {
            let name = part.file_name().unwrap();
            assert!(name.len() <= 255, "{}", name.len());
            assert!(name.to_string_lossy().ends_with(PART_SUFFIX));
            assert_eq!(part.parent(), Some(dir));
        }
        assert_ne!(pa, pb);
        assert_eq!(pa, part_path(&a));
        // A name with room for the suffix keeps the plain form.
        let fits = dir.join("x".repeat(243));
        assert_eq!(part_path(&fits).file_name().unwrap().len(), 255);
        assert!(
            part_path(&fits)
                .to_string_lossy()
                .ends_with("x.pairee-part")
        );
    }

    #[test]
    fn commit_replaces_read_only_files_and_refuses_folders() {
        let dir = tempfile::tempdir().unwrap();
        let dst = dir.path().join("a.txt");
        std::fs::write(&dst, b"old").unwrap();
        let mut perms = std::fs::metadata(&dst).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&dst, perms).unwrap();
        let part = part_path(&dst);
        std::fs::write(&part, b"new").unwrap();
        assert_eq!(commit(&part, &dst, false).unwrap(), Replaced::Overwritten);
        assert_eq!(std::fs::read(&dst).unwrap(), b"new");
        assert!(!part.exists());

        let folder = dir.path().join("folder");
        std::fs::create_dir(&folder).unwrap();
        let part = part_path(&folder);
        std::fs::write(&part, b"x").unwrap();
        assert!(commit(&part, &folder, false).is_err());
        assert!(folder.is_dir());
    }

    #[test]
    fn a_failed_atomic_write_keeps_the_original() {
        let dir = tempfile::tempdir().unwrap();
        let dst = dir.path().join("a.txt");
        std::fs::write(&dst, b"original").unwrap();
        let result = write_atomically(
            &dst,
            |part| {
                std::fs::write(part, b"half")?;
                Err(io::Error::other("cancelled"))
            },
            |from, to| std::fs::rename(from, to),
            discard,
        );
        assert!(result.is_err());
        assert_eq!(std::fs::read(&dst).unwrap(), b"original");
        assert!(!part_path(&dst).exists());
    }
}
