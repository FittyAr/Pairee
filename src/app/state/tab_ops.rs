//! Tab commands on the focused side: open, close, switch, move, lock and
//! rename. Every switch rereads the newly shown tab in the background.

use super::AppState;
use super::tabs::{PanelTabs, TabId};

impl AppState {
    /// Opens a copy of the focused tab next to it and shows it.
    pub fn duplicate_active_tab(&mut self, show_hidden: bool) {
        let side = self.panels.active;
        let tab = self.panels.tabs(side).active().duplicate();
        let id = self.panels.tabs_mut(side).insert_after(None, tab);
        self.refresh_tab(id, show_hidden, false);
    }

    /// Closes the focused tab; the last tab of a side stays.
    pub fn close_active_tab(&mut self, show_hidden: bool) -> bool {
        let id = self.panels.active_tab_id(self.panels.active);
        self.close_tab(id, show_hidden)
    }

    /// Closes tab `id` on whichever side it is.
    pub fn close_tab(&mut self, id: TabId, show_hidden: bool) -> bool {
        let Some((side, _)) = self.panels.find_tab(id) else {
            return false;
        };
        let shown = self.panels.active_tab_id(side) == id;
        let closed = self.panels.tabs_mut(side).close(id);
        if closed && shown {
            self.refresh_panel(side, show_hidden, false);
        }
        closed
    }

    /// Shows tab `id` and focuses its side.
    pub fn activate_tab(&mut self, id: TabId, show_hidden: bool) -> bool {
        let Some((side, _)) = self.panels.find_tab(id) else {
            return false;
        };
        self.panels.active = side;
        let changed = self.panels.tabs_mut(side).activate(id);
        self.after_tab_switch(changed, show_hidden)
    }

    /// Shows the tab at `at` (0-based) on the focused side.
    pub fn activate_tab_index(&mut self, at: usize, show_hidden: bool) -> bool {
        let changed = self.focused_tabs().activate_index(at);
        self.after_tab_switch(changed, show_hidden)
    }

    /// Shows the next (or previous) tab of the focused side.
    pub fn cycle_tab(&mut self, forward: bool, show_hidden: bool) -> bool {
        let changed = self.focused_tabs().cycle(forward);
        self.after_tab_switch(changed, show_hidden)
    }

    /// Moves the focused tab one place right (or left).
    pub fn move_active_tab(&mut self, forward: bool) -> bool {
        self.focused_tabs().move_active(forward)
    }

    pub fn toggle_active_tab_lock(&mut self) {
        self.focused_tabs().active_mut().toggle_lock();
    }

    /// Sets (or, with an empty `name`, clears) the title of tab `id`.
    pub fn rename_tab(&mut self, id: TabId, name: &str) {
        if let Some((_, tab)) = self.panels.find_tab_mut(id) {
            let name = name.trim();
            tab.name = (!name.is_empty()).then(|| name.to_string());
        }
    }

    fn focused_tabs(&mut self) -> &mut PanelTabs {
        self.panels.tabs_mut(self.panels.active)
    }

    fn after_tab_switch(&mut self, changed: bool, show_hidden: bool) -> bool {
        if changed {
            self.refresh_active_panel(show_hidden);
        }
        changed
    }

    /// A locked tab pointed at another folder goes back to its folder and
    /// the new location opens in a tab next to it. Returns `true` when the
    /// navigation was diverted.
    pub(super) fn divert_locked_tab(&mut self, id: TabId, show_hidden: bool) -> bool {
        let Some((side, tab)) = self.panels.find_tab_mut(id) else {
            return false;
        };
        let Some(lock) = tab.lock.clone().filter(|_| tab.left_its_lock()) else {
            return false;
        };
        let mut diverted = tab.duplicate();
        diverted.panel.pending_focus = tab.panel.pending_focus.take();
        diverted.panel.last_path = lock.path.clone();
        let target = std::mem::replace(&mut tab.panel.current_path, lock.path);
        tab.panel.source = lock.source;
        if let Some(at) = tab.panel.entries.iter().position(|e| e.path == target) {
            tab.panel.cursor_index = at;
        }
        let new_id = self.panels.tabs_mut(side).insert_after(Some(id), diverted);
        self.refresh_tab(new_id, show_hidden, false);
        true
    }
}
