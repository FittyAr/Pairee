use super::panel::PanelState;
use super::types::ActivePanel;
use std::path::PathBuf;

/// Dual file-panel pair plus visibility / focus flags.
pub struct PanelPair {
    pub left: PanelState,
    pub right: PanelState,
    pub active: ActivePanel,
    pub left_visible: bool,
    pub right_visible: bool,
    /// Ctrl+O: hide both panels to reveal the full terminal output below.
    pub both_hidden: bool,
    /// Whether quick-view is active (passive panel shows file preview).
    pub quick_view_active: bool,
}

impl PanelPair {
    pub fn new(left_path: PathBuf, right_path: PathBuf) -> Self {
        Self {
            left: PanelState::new(left_path),
            right: PanelState::new(right_path),
            active: ActivePanel::Left,
            left_visible: true,
            right_visible: true,
            both_hidden: false,
            quick_view_active: false,
        }
    }

    pub fn active(&self) -> &PanelState {
        match self.active {
            ActivePanel::Left => &self.left,
            ActivePanel::Right => &self.right,
        }
    }

    pub fn active_mut(&mut self) -> &mut PanelState {
        match self.active {
            ActivePanel::Left => &mut self.left,
            ActivePanel::Right => &mut self.right,
        }
    }

    pub fn passive(&self) -> &PanelState {
        match self.active {
            ActivePanel::Left => &self.right,
            ActivePanel::Right => &self.left,
        }
    }

    pub fn swap(&mut self) {
        std::mem::swap(&mut self.left, &mut self.right);
    }

    pub fn toggle_focus(&mut self) {
        self.active = match self.active {
            ActivePanel::Left => ActivePanel::Right,
            ActivePanel::Right => ActivePanel::Left,
        };
    }
}
