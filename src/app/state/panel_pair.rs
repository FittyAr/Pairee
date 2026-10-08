use super::panel::PanelState;
use super::tabs::{PanelTabs, Tab, TabId};
use super::types::ActivePanel;
use std::path::PathBuf;

/// Dual file-panel pair (each side a set of folder tabs) plus visibility /
/// focus flags. "The panel" of a side is its active tab's panel.
pub struct PanelPair {
    left: PanelTabs,
    right: PanelTabs,
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
            left: PanelTabs::new(PanelState::new(left_path)),
            right: PanelTabs::new(PanelState::new(right_path)),
            active: ActivePanel::Left,
            left_visible: true,
            right_visible: true,
            both_hidden: false,
            quick_view_active: false,
        }
    }

    pub fn active(&self) -> &PanelState {
        self.side(self.active)
    }

    pub fn active_mut(&mut self) -> &mut PanelState {
        self.side_mut(self.active)
    }

    /// The panel shown on `side` (its active tab).
    pub fn side(&self, side: ActivePanel) -> &PanelState {
        self.tabs(side).panel()
    }

    pub fn side_mut(&mut self, side: ActivePanel) -> &mut PanelState {
        self.tabs_mut(side).panel_mut()
    }

    pub fn tabs(&self, side: ActivePanel) -> &PanelTabs {
        match side {
            ActivePanel::Left => &self.left,
            ActivePanel::Right => &self.right,
        }
    }

    pub fn tabs_mut(&mut self, side: ActivePanel) -> &mut PanelTabs {
        match side {
            ActivePanel::Left => &mut self.left,
            ActivePanel::Right => &mut self.right,
        }
    }

    /// Every tab of both sides, with its side.
    pub fn all_tabs_mut(&mut self) -> impl Iterator<Item = (ActivePanel, &mut Tab)> {
        let left = self.left.tabs_mut().map(|tab| (ActivePanel::Left, tab));
        let right = self.right.tabs_mut().map(|tab| (ActivePanel::Right, tab));
        left.chain(right)
    }

    /// Tab `id` and its side, wherever it is (`None` once it was closed).
    pub fn find_tab(&self, id: TabId) -> Option<(ActivePanel, &Tab)> {
        [ActivePanel::Left, ActivePanel::Right]
            .into_iter()
            .find_map(|side| self.tabs(side).find(id).map(|tab| (side, tab)))
    }

    pub fn find_tab_mut(&mut self, id: TabId) -> Option<(ActivePanel, &mut Tab)> {
        self.all_tabs_mut().find(|(_, tab)| tab.id == id)
    }

    /// Id of the tab shown on `side`.
    pub fn active_tab_id(&self, side: ActivePanel) -> TabId {
        self.tabs(side).active().id
    }

    pub fn passive(&self) -> &PanelState {
        self.side(self.active.other())
    }

    /// Exchanges the two sides, tabs included.
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.left, &mut self.right);
    }

    pub fn toggle_focus(&mut self) {
        self.active = self.active.other();
    }
}
