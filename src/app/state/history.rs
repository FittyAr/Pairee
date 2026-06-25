use super::AppState;
use std::path::PathBuf;

impl AppState {
    /// Pushes a path to the file view history.
    pub fn push_file_view_history(&mut self, path: PathBuf) {
        let mut store = crate::config::history::HistoryStore {
            commands: std::mem::take(&mut self.command_history),
            viewed_files: std::mem::take(&mut self.file_view_history),
            visited_folders: std::mem::take(&mut self.folders_history),
        };
        store.push_viewed_file(path);
        self.command_history = store.commands;
        self.file_view_history = store.viewed_files;
        self.folders_history = store.visited_folders;
    }

    /// Pushes a folder to the folders history.
    pub fn push_folders_history(&mut self, path: PathBuf) {
        let mut store = crate::config::history::HistoryStore {
            commands: std::mem::take(&mut self.command_history),
            viewed_files: std::mem::take(&mut self.file_view_history),
            visited_folders: std::mem::take(&mut self.folders_history),
        };
        store.push_visited_folder(path);
        self.command_history = store.commands;
        self.file_view_history = store.viewed_files;
        self.folders_history = store.visited_folders;
    }

    /// Pushes a CLI command to the command history.
    pub fn push_command_history(&mut self, cmd: String) {
        let mut store = crate::config::history::HistoryStore {
            commands: std::mem::take(&mut self.command_history),
            viewed_files: std::mem::take(&mut self.file_view_history),
            visited_folders: std::mem::take(&mut self.folders_history),
        };
        store.push_command(cmd);
        self.command_history = store.commands;
        self.file_view_history = store.viewed_files;
        self.folders_history = store.visited_folders;
    }
}
