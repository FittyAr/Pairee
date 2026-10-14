use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_HISTORY: usize = 100;

/// Persists the history lists between sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HistoryStore {
    /// CLI commands executed in the command-line bar.
    pub commands: Vec<String>,
    /// Files opened with the F3 viewer or F4 editor.
    pub viewed_files: Vec<PathBuf>,
    /// Directories navigated to in the panels.
    pub visited_folders: Vec<PathBuf>,
    /// Patterns searched for in the editor (find / replace dialog).
    #[serde(default)]
    pub searches: Vec<String>,
    /// Replacement texts of the editor's replace dialog.
    #[serde(default)]
    pub replacements: Vec<String>,
}

/// Puts `item` at the front of `list`, dropping an older copy and the
/// entries past [`MAX_HISTORY`].
fn push_front<T: PartialEq>(list: &mut Vec<T>, item: T) {
    list.retain(|old| old != &item);
    list.insert(0, item);
    list.truncate(MAX_HISTORY);
}

impl HistoryStore {
    /// Loads history from `<cache_dir>/history.toml`, returning a default on a missing or invalid file.
    pub fn load() -> Self {
        super::toml_store::load_or_default(&history_path())
    }

    /// Saves what this instance changed since `base` (what it loaded or
    /// last saved) on top of `history.toml` as it is now, so entries other
    /// running instances saved meanwhile are kept (see
    /// [`super::shared_file::merge_recent`]). `keep` filters what is
    /// written (the lists the user does not keep stay out).
    pub fn save_merged(
        &self,
        base: &HistoryStore,
        keep: impl FnOnce(HistoryStore) -> HistoryStore,
    ) -> Result<HistoryStore> {
        super::shared_file::merge_save(&history_path(), |disk: Option<HistoryStore>| {
            keep(self.merged(base, &disk.unwrap_or_default()))
        })
    }

    /// This instance's changes from `base` applied to `disk`, list by list.
    pub fn merged(&self, base: &HistoryStore, disk: &HistoryStore) -> HistoryStore {
        use super::shared_file::merge_recent;
        HistoryStore {
            commands: merge_recent(&base.commands, &self.commands, &disk.commands, MAX_HISTORY),
            viewed_files: merge_recent(
                &base.viewed_files,
                &self.viewed_files,
                &disk.viewed_files,
                MAX_HISTORY,
            ),
            visited_folders: merge_recent(
                &base.visited_folders,
                &self.visited_folders,
                &disk.visited_folders,
                MAX_HISTORY,
            ),
            searches: merge_recent(&base.searches, &self.searches, &disk.searches, MAX_HISTORY),
            replacements: merge_recent(
                &base.replacements,
                &self.replacements,
                &disk.replacements,
                MAX_HISTORY,
            ),
        }
    }

    /// Clears the lists whose "save … history" setting is off, so only the
    /// categories the user opted into are restored or written to disk.
    /// The editor's search and replace lists go with the "view and edit"
    /// history.
    pub fn retain_enabled(mut self, commands: bool, folders: bool, viewed_files: bool) -> Self {
        if !commands {
            self.commands.clear();
        }
        if !folders {
            self.visited_folders.clear();
        }
        if !viewed_files {
            self.viewed_files.clear();
            self.searches.clear();
            self.replacements.clear();
        }
        self
    }

    /// Adds a command to the front of the list, removing duplicates and capping at MAX_HISTORY.
    pub fn push_command(&mut self, cmd: impl Into<String>) {
        let cmd = cmd.into();
        if !cmd.trim().is_empty() {
            push_front(&mut self.commands, cmd);
        }
    }

    /// Adds a viewed file to the front of the list.
    pub fn push_viewed_file(&mut self, path: PathBuf) {
        push_front(&mut self.viewed_files, path);
    }

    /// Adds a visited folder to the front of the list.
    pub fn push_visited_folder(&mut self, path: PathBuf) {
        push_front(&mut self.visited_folders, path);
    }

    /// Adds an editor search pattern and, when given, its replacement.
    pub fn push_search(&mut self, pattern: String, replacement: Option<String>) {
        if !pattern.is_empty() {
            push_front(&mut self.searches, pattern);
        }
        if let Some(replacement) = replacement {
            push_front(&mut self.replacements, replacement);
        }
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
        store.push_search("a".into(), Some(String::new()));
        assert!(
            store
                .clone()
                .retain_enabled(true, true, false)
                .searches
                .is_empty()
        );
        let all = store.retain_enabled(true, true, true);
        assert_eq!((all.searches.len(), all.replacements.len()), (1, 1));
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

    #[test]
    fn older_files_without_search_lists_still_load() {
        let old = "commands = [\"ls\"]\nviewed_files = []\nvisited_folders = []\n";
        let store: HistoryStore = toml::from_str(old).unwrap();
        assert_eq!(store.commands, ["ls"]);
        assert!(store.searches.is_empty());
    }
}
