//! A validated `pairee.fs` path and the operations on it.
//!
//! Jailed targets are opened through a `cap_std::fs::Dir` handle of the
//! jail root they were validated against: every component of the relative
//! path is resolved beneath that handle (`openat` + `O_NOFOLLOW`-style
//! resolution on Unix, handle-relative resolution on Windows), and any
//! symlink, junction or `..` that would leave the root is refused at use
//! time. A component swapped for an escaping link between the check and the
//! operation therefore makes the operation fail instead of escaping.

use cap_std::fs::Dir;
use std::io;
use std::path::{Path, PathBuf};

/// Result of `FsPolicy::check`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Unrestricted (trusted plugin outside Secure Mode): plain `std::fs`.
    Free(PathBuf),
    /// Inside a jail root: `root` is canonical, `rel` is relative to it and
    /// `path` is the lexical path shown to Lua.
    Jailed {
        root: PathBuf,
        rel: PathBuf,
        path: PathBuf,
    },
}

/// `pairee.fs.remove` variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoveKind {
    File,
    Dir,
    DirAll,
    DirClean,
}

impl RemoveKind {
    pub fn from_lua_name(kind: &str) -> Self {
        match kind {
            "dir" => Self::Dir,
            "dir_all" => Self::DirAll,
            "dir_clean" => Self::DirClean,
            _ => Self::File,
        }
    }
}

impl Target {
    /// Path to show to Lua / build `File` userdata from.
    pub fn path(&self) -> &Path {
        match self {
            Self::Free(path) | Self::Jailed { path, .. } => path,
        }
    }

    /// Run `free` on the plain path or `jailed` on (root handle, relative).
    fn with<T>(
        &self,
        free: impl FnOnce(&Path) -> io::Result<T>,
        jailed: impl FnOnce(&Dir, &Path) -> io::Result<T>,
    ) -> io::Result<T> {
        blocking(|| match self {
            Self::Free(path) => free(path),
            Self::Jailed { root, rel, .. } => jailed(&open_root(root)?, rel_or_dot(rel)),
        })
    }

    pub fn read_to_string(&self) -> io::Result<String> {
        self.with(|p| std::fs::read_to_string(p), |d, r| d.read_to_string(r))
    }

    pub fn write(&self, data: &str) -> io::Result<()> {
        self.with(|p| std::fs::write(p, data), |d, r| d.write(r, data))
    }

    pub fn exists(&self) -> bool {
        self.with(|p| Ok(p.exists()), |d, r| Ok(d.exists(r)))
            .unwrap_or(false)
    }

    pub fn create_dir(&self, all: bool) -> io::Result<()> {
        self.with(
            |p| {
                if all {
                    std::fs::create_dir_all(p)
                } else {
                    std::fs::create_dir(p)
                }
            },
            |d, r| {
                if all {
                    d.create_dir_all(r)
                } else {
                    d.create_dir(r)
                }
            },
        )
    }

    pub fn remove(&self, kind: RemoveKind) -> io::Result<()> {
        self.with(
            |p| match kind {
                RemoveKind::File => std::fs::remove_file(p),
                RemoveKind::Dir => std::fs::remove_dir(p),
                RemoveKind::DirAll => std::fs::remove_dir_all(p),
                RemoveKind::DirClean => {
                    clean_dir(&Dir::open_ambient_dir(p, cap_std::ambient_authority())?)
                }
            },
            |d, r| match kind {
                RemoveKind::File => d.remove_file(r),
                RemoveKind::Dir => d.remove_dir(r),
                RemoveKind::DirAll => d.remove_dir_all(r),
                RemoveKind::DirClean => clean_dir(&d.open_dir(r)?),
            },
        )
    }

    /// Entry paths of a directory (`path().join(name)`); empty on error.
    pub fn list(&self) -> Vec<PathBuf> {
        let names = self.with(
            |p| {
                std::fs::read_dir(p)?
                    .map(|e| e.map(|e| e.file_name()))
                    .collect::<io::Result<Vec<_>>>()
            },
            |d, r| {
                d.read_dir(r)?
                    .map(|e| e.map(|e| e.file_name()))
                    .collect::<io::Result<Vec<_>>>()
            },
        );
        names
            .unwrap_or_default()
            .into_iter()
            .map(|name| self.path().join(name))
            .collect()
    }

    pub fn rename_to(&self, to: &Target) -> io::Result<()> {
        self.with_pair(
            to,
            |f, t| std::fs::rename(f, t),
            |fd, fr, td, tr| fd.rename(fr, td, tr),
        )
    }

    pub fn copy_to(&self, to: &Target) -> io::Result<u64> {
        self.with_pair(
            to,
            |f, t| std::fs::copy(f, t),
            |fd, fr, td, tr| fd.copy(fr, td, tr),
        )
    }

    fn with_pair<T>(
        &self,
        to: &Target,
        free: impl FnOnce(&Path, &Path) -> io::Result<T>,
        jailed: impl FnOnce(&Dir, &Path, &Dir, &Path) -> io::Result<T>,
    ) -> io::Result<T> {
        blocking(|| match (self, to) {
            (Self::Free(from), Self::Free(to)) => free(from, to),
            (
                Self::Jailed {
                    root: from_root,
                    rel: from_rel,
                    ..
                },
                Self::Jailed {
                    root: to_root,
                    rel: to_rel,
                    ..
                },
            ) => jailed(
                &open_root(from_root)?,
                rel_or_dot(from_rel),
                &open_root(to_root)?,
                rel_or_dot(to_rel),
            ),
            _ => Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "mixed jailed and unrestricted paths",
            )),
        })
    }
}

fn open_root(root: &Path) -> io::Result<Dir> {
    Dir::open_ambient_dir(root, cap_std::ambient_authority())
}

fn rel_or_dot(rel: &Path) -> &Path {
    if rel.as_os_str().is_empty() {
        Path::new(".")
    } else {
        rel
    }
}

/// Remove every entry of `dir`, keeping `dir` itself.
fn clean_dir(dir: &Dir) -> io::Result<()> {
    for entry in dir.entries()? {
        let entry = entry?;
        let name = entry.file_name();
        if entry.file_type()?.is_dir() {
            dir.remove_dir_all(&name)?;
        } else {
            dir.remove_file(&name)?;
        }
    }
    Ok(())
}

/// Let other tasks run on a multi-thread runtime while blocking on I/O.
fn blocking<T>(f: impl FnOnce() -> T) -> T {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(f)
        }
        _ => f(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jailed(root: &Path, rel: &str) -> Target {
        let root = std::fs::canonicalize(root).unwrap();
        Target::Jailed {
            path: root.join(rel),
            root,
            rel: PathBuf::from(rel),
        }
    }

    #[test]
    fn jailed_ops_work_inside_root() {
        let dir = tempfile::tempdir().unwrap();
        jailed(dir.path(), "sub").create_dir(true).unwrap();
        let file = jailed(dir.path(), "sub/a.txt");
        file.write("hi").unwrap();
        assert_eq!(file.read_to_string().unwrap(), "hi");
        assert!(file.exists());
        let copy = jailed(dir.path(), "sub/b.txt");
        assert_eq!(file.copy_to(&copy).unwrap(), 2);
        copy.rename_to(&jailed(dir.path(), "c.txt")).unwrap();
        assert_eq!(jailed(dir.path(), "sub").list().len(), 1);
        jailed(dir.path(), "sub")
            .remove(RemoveKind::DirClean)
            .unwrap();
        assert!(dir.path().join("sub").is_dir());
        assert!(!dir.path().join("sub/a.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_swapped_in_after_check_cannot_escape() {
        let jail = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), "s3cret").unwrap();
        // Validation saw a plain directory `d`...
        std::fs::create_dir(jail.path().join("d")).unwrap();
        let target = jailed(jail.path(), "d/secret");
        let out_file = jailed(jail.path(), "d/pwn");
        // ...which is swapped for a link to the outside before use.
        std::fs::remove_dir(jail.path().join("d")).unwrap();
        std::os::unix::fs::symlink(outside.path(), jail.path().join("d")).unwrap();
        assert!(target.read_to_string().is_err());
        assert!(out_file.write("x").is_err());
        assert!(!outside.path().join("pwn").exists());
    }

    #[cfg(unix)]
    #[test]
    fn final_component_symlink_to_outside_is_refused() {
        let jail = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let victim = outside.path().join("victim");
        std::fs::write(&victim, "keep").unwrap();
        let target = jailed(jail.path(), "f");
        std::os::unix::fs::symlink(&victim, jail.path().join("f")).unwrap();
        assert!(target.write("pwned").is_err());
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "keep");
    }

    #[cfg(windows)]
    #[test]
    fn junction_swapped_in_after_check_cannot_escape() {
        let jail = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), "s3cret").unwrap();
        std::fs::create_dir(jail.path().join("d")).unwrap();
        let target = jailed(jail.path(), "d\\secret");
        let out_file = jailed(jail.path(), "d\\pwn");
        std::fs::remove_dir(jail.path().join("d")).unwrap();
        // Directory symlinks need a privilege; junctions do not.
        let status = std::process::Command::new("cmd.exe")
            .args(["/C", "mklink", "/J"])
            .arg(jail.path().join("d"))
            .arg(outside.path())
            .output()
            .unwrap();
        assert!(status.status.success(), "{status:?}");
        assert!(target.read_to_string().is_err());
        assert!(out_file.write("x").is_err());
        assert!(!outside.path().join("pwn").exists());
    }

    #[test]
    fn dotdot_in_relative_part_is_refused_at_use() {
        let jail = tempfile::tempdir().unwrap();
        let target = jailed(jail.path(), "../escape.txt");
        assert!(target.write("x").is_err());
    }
}
