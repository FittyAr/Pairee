use super::AppState;
use super::types::Screen;

impl AppState {
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
