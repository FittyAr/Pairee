use super::AppState;
use crate::config::history::HistoryStore;
use std::path::PathBuf;

/// In-memory command / file / folder history (persisted via `HistoryStore`).
#[derive(Debug, Default, Clone)]
pub struct HistoryState {
    pub commands: Vec<String>,
    pub viewed_files: Vec<PathBuf>,
    pub folders: Vec<PathBuf>,
}

impl HistoryState {
    pub fn from_store(store: HistoryStore) -> Self {
        Self {
            commands: store.commands,
            viewed_files: store.viewed_files,
            folders: store.visited_folders,
        }
    }

    pub fn to_store(&self) -> HistoryStore {
        HistoryStore {
            commands: self.commands.clone(),
            viewed_files: self.viewed_files.clone(),
            visited_folders: self.folders.clone(),
        }
    }

    fn mutate<F: FnOnce(&mut HistoryStore)>(&mut self, f: F) {
        let mut store = HistoryStore {
            commands: std::mem::take(&mut self.commands),
            viewed_files: std::mem::take(&mut self.viewed_files),
            visited_folders: std::mem::take(&mut self.folders),
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
}

impl AppState {
    /// Pushes a path to the file view history.
    pub fn push_file_view_history(&mut self, path: PathBuf) {
        self.history.push_viewed_file(path);
    }

    /// Pushes a folder to the folders history.
    pub fn push_folders_history(&mut self, path: PathBuf) {
        self.history.push_visited_folder(path);
    }

    /// Pushes a CLI command to the command history.
    pub fn push_command_history(&mut self, cmd: String) {
        self.history.push_command(cmd);
    }
}
