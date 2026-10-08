use super::AppState;
use super::types::Screen;
use crate::ui::viewer::{ViewerMode, ViewerState};

impl AppState {
    /// Opens the internal viewer on `path`. The file is read by a background
    /// job; a "Loading…" viewer is shown meanwhile.
    pub fn open_viewer(&mut self, path: std::path::PathBuf, allow_image: bool, hex: bool) {
        self.push_screen(Screen::Viewer(ViewerState::loading(path.clone())));
        self.viewer_load.start(move |_| {
            let mut viewer = ViewerState::load_with_images(path, allow_image);
            if hex {
                viewer.mode = ViewerMode::Hex;
            }
            viewer
        });
        self.poll_viewer_load();
    }

    /// Replaces the loading placeholder with the finished viewer.
    pub fn poll_viewer_load(&mut self) -> bool {
        let Some(loaded) = self.viewer_load.poll() else {
            return false;
        };
        let slot = self.screens.iter_mut().find_map(|s| match s {
            Screen::Viewer(v) if v.loading && v.path == loaded.path => Some(v),
            _ => None,
        });
        if let Some(viewer) = slot {
            *viewer = loaded;
            self.mark_ui_dirty();
        }
        true
    }

    /// Adds a new screen to the stack and makes it active.
    pub fn push_screen(&mut self, screen: Screen) {
        if self.active_screen_idx < self.screen_dialogs.len() {
            self.screen_dialogs[self.active_screen_idx] = std::mem::take(&mut self.dialogs);
        }
        self.screens.push(screen);
        self.screen_dialogs.push(super::DialogStack::new());
        self.active_screen_idx = self.screens.len() - 1;
        self.dialogs.clear();
    }

    /// Switches to the next screen (Ctrl-Tab).
    pub fn next_screen(&mut self) {
        if self.screens.len() > 1 {
            self.screen_dialogs[self.active_screen_idx] = std::mem::take(&mut self.dialogs);
            self.active_screen_idx = (self.active_screen_idx + 1) % self.screens.len();
            self.dialogs = std::mem::take(&mut self.screen_dialogs[self.active_screen_idx]);
        }
    }

    /// Switches to the previous screen (Ctrl-Shift-Tab).
    pub fn prev_screen(&mut self) {
        if self.screens.len() > 1 {
            self.screen_dialogs[self.active_screen_idx] = std::mem::take(&mut self.dialogs);
            self.active_screen_idx = if self.active_screen_idx == 0 {
                self.screens.len() - 1
            } else {
                self.active_screen_idx - 1
            };
            self.dialogs = std::mem::take(&mut self.screen_dialogs[self.active_screen_idx]);
        }
    }

    /// Closes the currently active screen, reverting to the previous one.
    pub fn close_current_screen(&mut self) {
        if self.active_screen_idx > 0 && self.active_screen_idx < self.screens.len() {
            self.screens.remove(self.active_screen_idx);
            self.screen_dialogs.remove(self.active_screen_idx);
            self.active_screen_idx -= 1;
            self.dialogs = std::mem::take(&mut self.screen_dialogs[self.active_screen_idx]);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::{AppState, Screen};

    #[test]
    fn open_viewer_replaces_loading_placeholder() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.txt");
        std::fs::write(&path, "hello").unwrap();
        let mut state = AppState::new(dir.path().into(), dir.path().into());
        state.open_viewer(path, false, true);
        match state.screens.last() {
            Some(Screen::Viewer(v)) => {
                assert!(!v.loading);
                assert_eq!(v.lines, vec!["hello".to_string()]);
                assert_eq!(v.mode, crate::ui::viewer::ViewerMode::Hex);
            }
            _ => panic!("viewer screen expected"),
        }
    }
}
