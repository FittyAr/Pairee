use super::types::PluginsLock;
use std::path::PathBuf;

fn get_lockfile_path() -> PathBuf {
    crate::config::paths::get_config_dir().join("plugins.lock")
}

pub fn read_lockfile() -> PluginsLock {
    let path = get_lockfile_path();
    if path.exists()
        && let Ok(content) = std::fs::read_to_string(&path)
        && let Ok(lock) = toml::from_str(&content)
    {
        return lock;
    }
    PluginsLock::default()
}

pub fn write_lockfile(lock: &PluginsLock) -> anyhow::Result<()> {
    let path = get_lockfile_path();
    let content = toml::to_string_pretty(lock)?;
    std::fs::write(&path, content)?;
    Ok(())
}
