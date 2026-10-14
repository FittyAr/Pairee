//! `plugins.lock`: the installed plugins, their versions, pins and the
//! SHA-256 of each file.
//!
//! A lockfile that exists but cannot be parsed is an **error**, never an
//! empty lock: `verify` would otherwise report success for nothing and the
//! next write would silently drop every pin. [`write_lockfile_to`] also
//! sets a corrupt file aside (`plugins.lock.bak`) instead of replacing it.

use super::types::PluginsLock;
use anyhow::Context as _;
use std::path::{Path, PathBuf};

fn get_lockfile_path() -> PathBuf {
    crate::config::paths::get_config_dir().join("plugins.lock")
}

/// The lockfile for display: an unreadable one shows as empty (and is
/// logged). Anything that decides or writes uses [`try_read_lockfile`].
pub fn read_lockfile() -> PluginsLock {
    try_read_lockfile().unwrap_or_else(|e| {
        tracing::warn!("{e:#}");
        PluginsLock::default()
    })
}

/// The lockfile; a missing file is an empty lock, a corrupt one an error.
pub fn try_read_lockfile() -> anyhow::Result<PluginsLock> {
    read_lockfile_from(&get_lockfile_path())
}

pub fn write_lockfile(lock: &PluginsLock) -> anyhow::Result<()> {
    write_lockfile_to(&get_lockfile_path(), lock)
}

/// Reads a lockfile: a missing file is an empty lock, an unreadable or
/// unparsable one an error that names the file.
pub fn read_lockfile_from(path: &Path) -> anyhow::Result<PluginsLock> {
    if !path.exists() {
        return Ok(PluginsLock::default());
    }
    let content =
        std::fs::read_to_string(path).with_context(|| format!("Cannot read {}", path.display()))?;
    toml::from_str(&content).with_context(|| {
        format!(
            "{} is corrupt; fix or delete it (installed plugins would otherwise be unpinned and unverified)",
            path.display()
        )
    })
}

pub fn write_lockfile_to(path: &Path, lock: &PluginsLock) -> anyhow::Result<()> {
    let content = toml::to_string_pretty(lock)?;
    if path.exists() && read_lockfile_from(path).is_err() {
        // Keep what the user had: the new lock replaces a file nobody can read.
        let backup = path.with_extension("lock.bak");
        std::fs::rename(path, &backup)
            .with_context(|| format!("Cannot set aside the corrupt {}", path.display()))?;
        tracing::warn!(
            "Corrupt {} was moved to {}",
            path.display(),
            backup.display()
        );
    }
    crate::config::write_atomic(path, content.as_bytes())
}
