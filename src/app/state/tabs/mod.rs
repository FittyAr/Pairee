//! Folder tabs of one panel side: an ordered list with one active tab.
//! The rest of the application keeps working on "the panel" of a side,
//! which is the active tab's [`PanelState`].

pub mod spec;
mod tab;
#[cfg(test)]
mod tests;

pub use tab::{PendingRemote, Tab, TabId};

use super::PanelState;

#[derive(Debug)]
pub struct PanelTabs {
    tabs: Vec<Tab>,
    active: usize,
}

impl PanelTabs {
    pub fn new(panel: PanelState) -> Self {
        Self {
            tabs: vec![Tab::new(panel)],
            active: 0,
        }
    }

    /// A side made of `tabs` (at least one) showing the tab at `active`
    /// (clamped to the last tab).
    pub fn from_tabs(tabs: Vec<Tab>, active: usize) -> Option<Self> {
        let last = tabs.len().checked_sub(1)?;
        Some(Self {
            tabs,
            active: active.min(last),
        })
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn tabs_mut(&mut self) -> impl Iterator<Item = &mut Tab> {
        self.tabs.iter_mut()
    }

    pub fn count(&self) -> usize {
        self.tabs.len()
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active(&self) -> &Tab {
        &self.tabs[self.active]
    }

    pub fn active_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }

    pub fn panel(&self) -> &PanelState {
        &self.active().panel
    }

    pub fn panel_mut(&mut self) -> &mut PanelState {
        &mut self.active_mut().panel
    }

    fn index_of(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == id)
    }

    pub fn find(&self, id: TabId) -> Option<&Tab> {
        self.tabs.iter().find(|tab| tab.id == id)
    }

    pub fn find_mut(&mut self, id: TabId) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|tab| tab.id == id)
    }

    /// Inserts `tab` right after tab `after` (or the active one) and
    /// activates it. Returns its id.
    pub fn insert_after(&mut self, after: Option<TabId>, tab: Tab) -> TabId {
        let at = after
            .and_then(|id| self.index_of(id))
            .unwrap_or(self.active)
            + 1;
        let id = tab.id;
        self.tabs.insert(at, tab);
        self.active = at;
        id
    }

    /// Closes tab `id`, cancelling its background jobs. The last tab of a
    /// side cannot be closed. Returns `true` when a tab was closed.
    pub fn close(&mut self, id: TabId) -> bool {
        if self.tabs.len() < 2 {
            return false;
        }
        let Some(at) = self.index_of(id) else {
            return false;
        };
        let mut tab = self.tabs.remove(at);
        tab.panel.cancel_jobs();
        if at < self.active || self.active == self.tabs.len() {
            self.active -= 1;
        }
        true
    }

    /// Activates tab `id`. Returns `true` when the active tab changed.
    pub fn activate(&mut self, id: TabId) -> bool {
        self.index_of(id).is_some_and(|at| self.activate_index(at))
    }

    /// Activates the tab at `at` (0-based). Returns `true` when the active
    /// tab changed.
    pub fn activate_index(&mut self, at: usize) -> bool {
        if at >= self.tabs.len() || at == self.active {
            return false;
        }
        self.active = at;
        true
    }

    /// Activates the next (or previous) tab, wrapping around.
    pub fn cycle(&mut self, forward: bool) -> bool {
        let len = self.tabs.len();
        let at = if forward {
            (self.active + 1) % len
        } else {
            (self.active + len - 1) % len
        };
        self.activate_index(at)
    }

    /// Moves the active tab one place left or right (no wrapping).
    pub fn move_active(&mut self, forward: bool) -> bool {
        let to = if forward {
            self.active + 1
        } else {
            match self.active.checked_sub(1) {
                Some(to) => to,
                None => return false,
            }
        };
        if to >= self.tabs.len() {
            return false;
        }
        self.tabs.swap(self.active, to);
        self.active = to;
        true
    }
}
