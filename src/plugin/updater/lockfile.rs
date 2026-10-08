use super::types::PluginsLock;
use std::path::{Path, PathBuf};

fn get_lockfile_path() -> PathBuf {
    crate::config::paths::get_config_dir().join("plugins.lock")
}

pub fn read_lockfile() -> PluginsLock {
    read_lockfile_from(&get_lockfile_path())
}

pub fn write_lockfile(lock: &PluginsLock) -> anyhow::Result<()> {
    write_lockfile_to(&get_lockfile_path(), lock)
}

/// Reads a lockfile; a missing or unparsable file yields an empty lock.
pub fn read_lockfile_from(path: &Path) -> PluginsLock {
    if path.exists()
        && let Ok(content) = std::fs::read_to_string(path)
        && let Ok(lock) = toml::from_str(&content)
    {
        return lock;
    }
    PluginsLock::default()
}

pub fn write_lockfile_to(path: &Path, lock: &PluginsLock) -> anyhow::Result<()> {
    let content = toml::to_string_pretty(lock)?;
    std::fs::write(path, content)?;
    Ok(())
}
