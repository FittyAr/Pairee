//! Leftovers of earlier updates: a staged binary of an update that crashed,
//! the previous `pairee.exe` that Windows could not delete while it ran, and
//! the work folder of a launched installer.

use std::path::Path;
use std::time::{Duration, SystemTime};

/// Prefix of every file the updater stages next to the Pairee binary.
pub const STAGING_PREFIX: &str = ".pairee_update_";
/// Prefix of the updater's private folders in the system temp directory.
pub const WORK_DIR_PREFIX: &str = "pairee_update_";
/// Younger leftovers may still belong to an update in progress (another
/// Pairee window, an installer that is still running).
const MIN_AGE: Duration = Duration::from_secs(60 * 60);

/// Removes leftovers of earlier updates; best effort, errors are only logged
/// (a file still in use is retried next time).
pub fn remove_stale_files() {
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        remove_stale_in(&dir, STAGING_PREFIX, MIN_AGE);
    }
    remove_stale_in(&std::env::temp_dir(), WORK_DIR_PREFIX, MIN_AGE);
}

fn remove_stale_in(dir: &Path, prefix: &str, min_age: Duration) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().starts_with(prefix) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let age = meta
            .modified()
            .ok()
            .and_then(|m| now.duration_since(m).ok())
            .unwrap_or_default();
        if age < min_age {
            continue;
        }
        let path = entry.path();
        let removed = if meta.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        match removed {
            Ok(()) => log::info!("Removed update leftover {}", path.display()),
            Err(e) => log::debug!("Update leftover {} kept: {e}", path.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_only_old_entries_with_the_prefix() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".pairee_update_abc"), b"x").unwrap();
        std::fs::create_dir(dir.path().join(".pairee_update_dir")).unwrap();
        std::fs::write(dir.path().join(".pairee_update_dir/f"), b"x").unwrap();
        std::fs::write(dir.path().join("pairee"), b"keep").unwrap();

        remove_stale_in(dir.path(), STAGING_PREFIX, Duration::from_secs(3600));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 3);

        remove_stale_in(dir.path(), STAGING_PREFIX, Duration::ZERO);
        let left: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, ["pairee"]);
    }
}
