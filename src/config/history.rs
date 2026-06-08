use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_HISTORY: usize = 100;

/// Persists the three history lists between sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryStore {
    /// CLI commands executed in the command-line bar.
    pub commands: Vec<String>,
    /// Files opened with the F3 viewer or F4 editor.
    pub viewed_files: Vec<PathBuf>,
    /// Directories navigated to in the panels.
    pub visited_folders: Vec<PathBuf>,
}

impl HistoryStore {
    /// Loads history from `<cache_dir>/ncrust/history.toml`, returning a default on missing file.
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(store) => store,
            Err(_) => Self::default(),
        }
    }

    fn try_load() -> Result<Self> {
        let path = history_path();
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Reading history file {:?}", path))?;
        toml::from_str(&content).context("Deserializing history.toml")
    }

    /// Persists the history to `<cache_dir>/ncrust/history.toml`.
    pub fn save(&self) -> Result<()> {
        let path = history_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Creating cache directory")?;
        }
        let toml_str = toml::to_string_pretty(self).context("Serializing history")?;
        std::fs::write(&path, toml_str)
            .with_context(|| format!("Writing history file {:?}", path))
    }

    /// Adds a command to the front of the list, removing duplicates and capping at MAX_HISTORY.
    pub fn push_command(&mut self, cmd: impl Into<String>) {
        let cmd = cmd.into();
        if !cmd.trim().is_empty() {
            self.commands.retain(|c| c != &cmd);
            self.commands.insert(0, cmd);
            self.commands.truncate(MAX_HISTORY);
        }
    }

    /// Adds a viewed file to the front of the list.
    pub fn push_viewed_file(&mut self, path: PathBuf) {
        self.viewed_files.retain(|p| p != &path);
        self.viewed_files.insert(0, path);
        self.viewed_files.truncate(MAX_HISTORY);
    }

    /// Adds a visited folder to the front of the list.
    pub fn push_visited_folder(&mut self, path: PathBuf) {
        self.visited_folders.retain(|p| p != &path);
        self.visited_folders.insert(0, path);
        self.visited_folders.truncate(MAX_HISTORY);
    }
}

fn history_path() -> PathBuf {
    crate::config::paths::get_cache_dir().join("history.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_command_deduplication() {
        let mut store = HistoryStore::default();
        store.push_command("ls");
        store.push_command("cd /tmp");
        store.push_command("ls");
        // "ls" should appear once, at the front
        assert_eq!(store.commands[0], "ls");
        assert_eq!(store.commands.len(), 2);
    }

    #[test]
    fn test_push_command_cap() {
        let mut store = HistoryStore::default();
        for i in 0..=MAX_HISTORY + 5 {
            store.push_command(format!("cmd_{}", i));
        }
        assert_eq!(store.commands.len(), MAX_HISTORY);
    }

    #[test]
    fn test_roundtrip_serialization() {
        let mut store = HistoryStore::default();
        store.push_command("cargo build");
        store.push_visited_folder(PathBuf::from("/home/user/projects"));
        let serialized = toml::to_string_pretty(&store).unwrap();
        let deserialized: HistoryStore = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.commands, store.commands);
        assert_eq!(deserialized.visited_folders, store.visited_folders);
    }
}
