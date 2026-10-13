//! The release files as they were before the release touched them, so a
//! failed gate or a cancelled release puts back exactly what the user had
//! (their own uncommitted edits included) instead of `git checkout`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{Result, ui};

#[derive(Default)]
pub struct Snapshot {
    /// Each file and its contents (`None`: it did not exist).
    files: Vec<(PathBuf, Option<Vec<u8>>)>,
    /// A folder the release renamed: `(from, to)`.
    moved: Option<(PathBuf, PathBuf)>,
}

impl Snapshot {
    pub fn take(paths: impl IntoIterator<Item = PathBuf>) -> Result<Self> {
        let mut files = Vec::new();
        for path in paths {
            let contents = match fs::read(&path) {
                Ok(bytes) => Some(bytes),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(format!("cannot read {}: {e}", path.display()).into()),
            };
            files.push((path, contents));
        }
        Ok(Self { files, moved: None })
    }

    /// Records that `from` was renamed to `to` (undone before the files are
    /// written back, so paths under `from` are valid again).
    pub fn record_move(&mut self, from: PathBuf, to: PathBuf) {
        if from != to {
            self.moved = Some((from, to));
        }
    }

    /// Puts every file back; reports what could not be restored.
    pub fn restore(&self) -> Result {
        let mut failures = Vec::new();
        if let Some((from, to)) = &self.moved
            && to.exists()
            && let Err(e) = fs::rename(to, from)
        {
            failures.push(format!(
                "move {} back to {}: {e}",
                to.display(),
                from.display()
            ));
        }
        for (path, contents) in &self.files {
            let restored = match contents {
                Some(bytes) => fs::write(path, bytes),
                None => remove_if_present(path),
            };
            if let Err(e) = restored {
                failures.push(format!("{}: {e}", path.display()));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!("could not restore:\n  {}", failures.join("\n  ")).into())
        }
    }

    /// [`restore`](Self::restore) for error paths: failures are printed.
    pub fn restore_or_warn(&self) {
        match self.restore() {
            Ok(()) => ui::ok("release files restored"),
            Err(e) => ui::error(&e.to_string()),
        }
    }
}

fn remove_if_present(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_undoes_edits_and_a_folder_move() {
        let root = std::env::temp_dir().join(format!("xtask-snapshot-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let from = root.join("1.0.0");
        let to = root.join("1.0.1");
        fs::create_dir_all(&from).unwrap();
        let manifest = from.join("m.yaml");
        let lock = root.join("Cargo.lock");
        fs::write(&manifest, "PackageVersion: 1.0.0").unwrap();
        fs::write(&lock, "user edit").unwrap();

        let mut snapshot =
            Snapshot::take([manifest.clone(), lock.clone(), root.join("new")]).unwrap();
        fs::write(&lock, "updated").unwrap();
        fs::write(&manifest, "PackageVersion: 1.0.1").unwrap();
        fs::rename(&from, &to).unwrap();
        snapshot.record_move(from.clone(), to.clone());
        fs::write(root.join("new"), "created").unwrap();

        snapshot.restore().unwrap();
        assert_eq!(fs::read_to_string(&lock).unwrap(), "user edit");
        assert_eq!(
            fs::read_to_string(&manifest).unwrap(),
            "PackageVersion: 1.0.0"
        );
        assert!(!to.exists());
        assert!(!root.join("new").exists());
        fs::remove_dir_all(&root).unwrap();
    }
}
