//! Minimal remote filesystem abstraction so recursive algorithms can be unit
//! tested without an SSH server.

use super::sftp_ops::{entry_name, is_real_child};
use anyhow::Result;
use ssh2::Sftp;
use std::path::{Path, PathBuf};

/// The SFTP primitives used by recursive deletion.
pub trait RemoteFs {
    /// `Some(is_dir)` when `path` exists, `None` otherwise.
    fn kind(&self, path: &Path) -> Option<bool>;
    /// Direct children as `(path, is_dir)`, without `.`/`..`.
    fn list(&self, dir: &Path) -> Result<Vec<(PathBuf, bool)>>;
    fn remove_file(&self, path: &Path) -> Result<()>;
    fn remove_dir(&self, path: &Path) -> Result<()>;
}

impl RemoteFs for Sftp {
    fn kind(&self, path: &Path) -> Option<bool> {
        // lstat: a symlink to a directory is removed as a link, never followed.
        self.lstat(path).ok().map(|s| s.is_dir())
    }

    fn list(&self, dir: &Path) -> Result<Vec<(PathBuf, bool)>> {
        Ok(self
            .readdir(dir)?
            .into_iter()
            .filter(|(p, _)| is_real_child(&entry_name(p)))
            .map(|(p, stat)| (p, stat.is_dir()))
            .collect())
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        Ok(self.unlink(path)?)
    }

    fn remove_dir(&self, path: &Path) -> Result<()> {
        Ok(self.rmdir(path)?)
    }
}

/// Deletes `path` and everything below it, children before parents
/// (post-order), so `rmdir` only ever runs on an already emptied directory.
pub fn delete_recursive(fs: &dyn RemoteFs, path: &Path) -> Result<()> {
    match fs.kind(path) {
        None => return Ok(()),
        Some(false) => return fs.remove_file(path),
        Some(true) => {}
    }
    // Iterative DFS: a directory is pushed back as "expanded" and removed
    // only after all of its subdirectories (pushed above it) are gone.
    let mut stack: Vec<(PathBuf, bool)> = vec![(path.to_path_buf(), false)];
    while let Some((dir, expanded)) = stack.pop() {
        if expanded {
            fs.remove_dir(&dir)?;
            continue;
        }
        stack.push((dir.clone(), true));
        for (child, is_dir) in fs.list(&dir)? {
            if is_dir {
                stack.push((child, false));
            } else {
                fs.remove_file(&child)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{RemoteFs, delete_recursive};
    use anyhow::{Result, bail};
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    /// In-memory tree enforcing POSIX `rmdir` semantics (must be empty).
    #[derive(Default)]
    struct FakeFs(RefCell<BTreeMap<PathBuf, bool>>);

    impl FakeFs {
        fn with(paths: &[(&str, bool)]) -> Self {
            let fs = Self::default();
            for (p, d) in paths {
                fs.0.borrow_mut().insert(PathBuf::from(p), *d);
            }
            fs
        }
    }

    impl RemoteFs for FakeFs {
        fn kind(&self, path: &Path) -> Option<bool> {
            self.0.borrow().get(path).copied()
        }
        fn list(&self, dir: &Path) -> Result<Vec<(PathBuf, bool)>> {
            Ok(self
                .0
                .borrow()
                .iter()
                .filter(|(p, _)| p.parent() == Some(dir))
                .map(|(p, d)| (p.clone(), *d))
                .collect())
        }
        fn remove_file(&self, path: &Path) -> Result<()> {
            self.0.borrow_mut().remove(path);
            Ok(())
        }
        fn remove_dir(&self, path: &Path) -> Result<()> {
            if !self.list(path)?.is_empty() {
                bail!("directory not empty: {}", path.display());
            }
            self.0.borrow_mut().remove(path);
            Ok(())
        }
    }

    #[test]
    fn nested_directories_are_removed_children_first() {
        let fs = FakeFs::with(&[
            ("/keep", true),
            ("/r", true),
            ("/r/a.txt", false),
            ("/r/sub", true),
            ("/r/sub/b.txt", false),
            ("/r/sub/deep", true),
            ("/r/sub/deep/c.txt", false),
            ("/r/sub2", true),
        ]);
        delete_recursive(&fs, Path::new("/r")).unwrap();
        let left: Vec<_> = fs.0.borrow().keys().cloned().collect();
        assert_eq!(left, vec![PathBuf::from("/keep")]);
    }

    #[test]
    fn single_file_and_missing_path() {
        let fs = FakeFs::with(&[("/f", false)]);
        delete_recursive(&fs, Path::new("/f")).unwrap();
        assert!(fs.0.borrow().is_empty());
        delete_recursive(&fs, Path::new("/missing")).unwrap();
    }
}
