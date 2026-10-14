use super::{AppState, PopupType};
use crate::app::state::popup::CommandHistoryList;
use crate::app::state::popup::FileViewHistoryList;
use crate::app::state::popup::FoldersHistoryList;
use crate::config::history::HistoryStore;
use std::path::PathBuf;

/// In-memory command / file / folder history (persisted via `HistoryStore`).
#[derive(Debug, Default, Clone)]
pub struct HistoryState {
    pub commands: Vec<String>,
    pub viewed_files: Vec<PathBuf>,
    pub folders: Vec<PathBuf>,
    /// Editor search patterns and replacements (newest first).
    pub searches: Vec<String>,
    pub replacements: Vec<String>,
    /// The lists as last loaded or saved: a save writes only what changed
    /// since, so other running instances' entries survive it.
    saved: HistoryStore,
}

/// One of the three history lists (Strategy for the shared history popup).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryKind {
    Commands,
    ViewedFiles,
    Folders,
}

impl HistoryKind {
    /// Identifier stored in `PopupType::ConfirmClearHistory`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Commands => "command",
            Self::ViewedFiles => "view",
            Self::Folders => "folder",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        [Self::Commands, Self::ViewedFiles, Self::Folders]
            .into_iter()
            .find(|kind| kind.key() == key)
    }

    /// The history popup for this list with the cursor on `cursor_idx`.
    pub fn list_popup(self, history: &HistoryState, cursor_idx: usize) -> PopupType {
        match self {
            Self::Commands => PopupType::CommandHistoryList(CommandHistoryList {
                entries: history.commands.clone(),
                cursor_idx,
            }),
            Self::ViewedFiles => PopupType::FileViewHistoryList(FileViewHistoryList {
                entries: history.viewed_files.clone(),
                cursor_idx,
            }),
            Self::Folders => PopupType::FoldersHistoryList(FoldersHistoryList {
                entries: history.folders.clone(),
                cursor_idx,
            }),
        }
    }
}

impl PopupType {
    /// Kind, cursor and length of an open history list popup.
    pub fn history_list_mut(&mut self) -> Option<(HistoryKind, &mut usize, usize)> {
        match self {
            PopupType::CommandHistoryList(CommandHistoryList {
                entries,
                cursor_idx,
            }) => Some((HistoryKind::Commands, cursor_idx, entries.len())),
            PopupType::FileViewHistoryList(FileViewHistoryList {
                entries,
                cursor_idx,
            }) => Some((HistoryKind::ViewedFiles, cursor_idx, entries.len())),
            PopupType::FoldersHistoryList(FoldersHistoryList {
                entries,
                cursor_idx,
            }) => Some((HistoryKind::Folders, cursor_idx, entries.len())),
            _ => None,
        }
    }
}

impl HistoryState {
    /// Writes the lists `settings` keep to disk (best effort: a failure is
    /// only logged). The others are never written, even mid-run. Only the
    /// changes since the last load or save are written over the file, so
    /// what other running instances saved meanwhile is kept.
    pub fn save(&mut self, settings: &crate::config::settings::Settings) {
        let kept = |store| crate::app::session::persisted_history(store, settings);
        let mine = self.to_store();
        match kept(mine.clone()).save_merged(&kept(self.saved.clone()), kept) {
            Ok(_) => self.saved = mine,
            Err(e) => log::warn!("Failed to save the history: {e}"),
        }
    }

    pub fn clear(&mut self, kind: HistoryKind) {
        match kind {
            HistoryKind::Commands => self.commands.clear(),
            HistoryKind::ViewedFiles => self.viewed_files.clear(),
            HistoryKind::Folders => self.folders.clear(),
        }
    }

    /// Removes entry `idx` of `kind`; returns the remaining length.
    pub fn remove(&mut self, kind: HistoryKind, idx: usize) -> usize {
        fn remove_at<T>(list: &mut Vec<T>, idx: usize) -> usize {
            if idx < list.len() {
                list.remove(idx);
            }
            list.len()
        }
        match kind {
            HistoryKind::Commands => remove_at(&mut self.commands, idx),
            HistoryKind::ViewedFiles => remove_at(&mut self.viewed_files, idx),
            HistoryKind::Folders => remove_at(&mut self.folders, idx),
        }
    }

    /// The lists of `store`, as loaded (the base of the next save).
    pub fn from_store(store: HistoryStore) -> Self {
        let mut state = Self {
            saved: store.clone(),
            ..Self::default()
        };
        state.set_lists(store);
        state
    }

    fn set_lists(&mut self, store: HistoryStore) {
        self.commands = store.commands;
        self.viewed_files = store.viewed_files;
        self.folders = store.visited_folders;
        self.searches = store.searches;
        self.replacements = store.replacements;
    }

    pub fn to_store(&self) -> HistoryStore {
        HistoryStore {
            commands: self.commands.clone(),
            viewed_files: self.viewed_files.clone(),
            visited_folders: self.folders.clone(),
            searches: self.searches.clone(),
            replacements: self.replacements.clone(),
        }
    }

    fn mutate<F: FnOnce(&mut HistoryStore)>(&mut self, f: F) {
        let mut store = HistoryStore {
            commands: std::mem::take(&mut self.commands),
            viewed_files: std::mem::take(&mut self.viewed_files),
            visited_folders: std::mem::take(&mut self.folders),
            searches: std::mem::take(&mut self.searches),
            replacements: std::mem::take(&mut self.replacements),
        };
        f(&mut store);
        self.set_lists(store);
    }

    pub fn push_viewed_file(&mut self, path: PathBuf) {
        self.mutate(|store| store.push_viewed_file(path));
    }

    pub fn push_visited_folder(&mut self, path: PathBuf) {
        self.mutate(|store| store.push_visited_folder(path));
    }

    pub fn push_command(&mut self, cmd: String) {
        self.mutate(|store| store.push_command(cmd));
    }

    pub fn push_search(&mut self, pattern: String, replacement: Option<String>) {
        self.mutate(|store| store.push_search(pattern, replacement));
    }
}

impl AppState {
    /// Pushes a path to the file view history.
    pub fn push_file_view_history(&mut self, path: PathBuf) {
        self.history.push_viewed_file(path);
    }

    /// Pushes a folder to the folders history. It keeps bare paths and is
    /// replayed on the local disk, so folders of a server are left out.
    pub fn push_folders_history(&mut self, path: PathBuf) {
        if self.get_active_panel().source.remote().is_none() {
            self.history.push_visited_folder(path);
        }
    }

    /// Pushes a CLI command to the command history.
    pub fn push_command_history(&mut self, cmd: String) {
        self.history.push_command(cmd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StateFile;

    #[test]
    fn saving_mid_run_skips_the_lists_the_user_does_not_keep() {
        let dir = tempfile::tempdir().unwrap();
        let _root = crate::config::paths::test_root::redirect(dir.path());
        let settings = crate::config::settings::Settings {
            save_commands_history: false,
            save_folders_history: true,
            ..Default::default()
        };
        let mut history = HistoryState {
            commands: vec!["mysql -p hunter2".into()],
            folders: vec![PathBuf::from("/kept")],
            ..Default::default()
        };

        history.save(&settings);

        let stored = HistoryStore::load();
        assert!(stored.commands.is_empty(), "{:?}", stored.commands);
        assert_eq!(stored.visited_folders.len(), 1);
    }

    #[test]
    fn two_running_instances_keep_each_others_history() {
        let dir = tempfile::tempdir().unwrap();
        let _root = crate::config::paths::test_root::redirect(dir.path());
        let settings = crate::config::settings::Settings::default();
        let mut start = HistoryState::default();
        start.push_command("shared".into());
        start.save(&settings);

        let mut first = HistoryState::from_store(HistoryStore::load());
        let mut second = HistoryState::from_store(HistoryStore::load());
        first.push_command("from first".into());
        second.push_command("from second".into());
        first.save(&settings);
        second.save(&settings);
        assert_eq!(
            HistoryStore::load().commands,
            ["from second", "from first", "shared"]
        );

        // A removal is one instance's change too: the other's entries stay.
        first.remove(HistoryKind::Commands, 1);
        first.save(&settings);
        assert_eq!(HistoryStore::load().commands, ["from second", "from first"]);
    }
}
