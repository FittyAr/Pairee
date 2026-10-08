use anyhow::Result;
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
    /// Loads history from `<cache_dir>/history.toml`, returning a default on a missing or invalid file.
    pub fn load() -> Self {
        super::toml_store::load_or_default(&history_path())
    }

    /// Persists the history to `<cache_dir>/history.toml`.
    pub fn save(&self) -> Result<()> {
        super::toml_store::save(&history_path(), self)
    }

    /// Clears the lists whose "save … history" setting is off, so only the
    /// categories the user opted into are restored or written to disk.
    pub fn retain_enabled(mut self, commands: bool, folders: bool, viewed_files: bool) -> Self {
        if !commands {
            self.commands.clear();
        }
        if !folders {
            self.visited_folders.clear();
        }
        if !viewed_files {
            self.viewed_files.clear();
        }
        self
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
    fn test_retain_enabled_clears_disabled_lists() {
        let mut store = HistoryStore::default();
        store.push_command("ls");
        store.push_visited_folder(PathBuf::from("/tmp"));
        store.push_viewed_file(PathBuf::from("/tmp/a.txt"));
        let kept = store.clone().retain_enabled(false, true, false);
        assert!(kept.commands.is_empty());
        assert_eq!(kept.visited_folders.len(), 1);
        assert!(kept.viewed_files.is_empty());
        let all = store.retain_enabled(true, true, true);
        assert_eq!(all.commands.len(), 1);
        assert_eq!(all.viewed_files.len(), 1);
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
