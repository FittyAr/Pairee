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
    /// only logged). The others are never written, even mid-run.
    pub fn save(&self, settings: &crate::config::settings::Settings) {
        let store = crate::app::session::persisted_history(self.to_store(), settings);
        if let Err(e) = store.save() {
            log::warn!("Failed to save the history: {e}");
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

    pub fn from_store(store: HistoryStore) -> Self {
        Self {
            commands: store.commands,
            viewed_files: store.viewed_files,
            folders: store.visited_folders,
            searches: store.searches,
            replacements: store.replacements,
        }
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
        *self = Self::from_store(store);
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

    #[test]
    fn saving_mid_run_skips_the_lists_the_user_does_not_keep() {
        let dir = tempfile::tempdir().unwrap();
        let _root = crate::config::paths::test_root::redirect(dir.path());
        let settings = crate::config::settings::Settings {
            save_commands_history: false,
            save_folders_history: true,
            ..Default::default()
        };
        let history = HistoryState {
            commands: vec!["mysql -p hunter2".into()],
            folders: vec![PathBuf::from("/kept")],
            ..Default::default()
        };

        history.save(&settings);

        let stored = HistoryStore::load();
        assert!(stored.commands.is_empty(), "{:?}", stored.commands);
        assert_eq!(stored.visited_folders.len(), 1);
    }
}
