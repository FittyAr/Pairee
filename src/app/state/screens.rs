use super::AppState;
use super::types::Screen;

impl AppState {
    /// Adds a new screen to the stack and makes it active.
    pub fn push_screen(&mut self, screen: Screen) {
        if self.active_screen_idx < self.screen_popups.len() {
            self.screen_popups[self.active_screen_idx] = self.active_popup.take();
        }
        self.screens.push(screen);
        self.screen_popups.push(None);
        self.active_screen_idx = self.screens.len() - 1;
        self.active_popup = None;
    }

    /// Switches to the next screen (Ctrl-Tab).
    pub fn next_screen(&mut self) {
        if self.screens.len() > 1 {
            self.screen_popups[self.active_screen_idx] = self.active_popup.take();
            self.active_screen_idx = (self.active_screen_idx + 1) % self.screens.len();
            self.active_popup = self.screen_popups[self.active_screen_idx].take();
        }
    }

    /// Switches to the previous screen (Ctrl-Shift-Tab).
    pub fn prev_screen(&mut self) {
        if self.screens.len() > 1 {
            self.screen_popups[self.active_screen_idx] = self.active_popup.take();
            self.active_screen_idx = if self.active_screen_idx == 0 {
                self.screens.len() - 1
            } else {
                self.active_screen_idx - 1
            };
            self.active_popup = self.screen_popups[self.active_screen_idx].take();
        }
    }

    /// Closes the currently active screen, reverting to the previous one.
    pub fn close_current_screen(&mut self) {
        if self.active_screen_idx > 0 && self.active_screen_idx < self.screens.len() {
            self.screens.remove(self.active_screen_idx);
            self.screen_popups.remove(self.active_screen_idx);
            self.active_screen_idx -= 1;
            self.active_popup = self.screen_popups[self.active_screen_idx].take();
        }
    }
}
