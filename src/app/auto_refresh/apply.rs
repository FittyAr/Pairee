//! Auto-refresh on the application state: which folders to monitor and
//! how a coalesced change reaches the tabs showing that folder.

use super::WatchTarget;
use crate::app::state::{ActivePanel, AppState, TabId};
use crate::fs::watch::DirChange;
use std::time::Instant;

impl AppState {
    /// Folders shown by the active tab of each side, when the tab is on the
    /// local disk (archive and SFTP tabs are not monitored).
    pub fn auto_refresh_targets(&self) -> Vec<WatchTarget> {
        let limit = self.disable_panel_update_object_count as usize;
        [ActivePanel::Left, ActivePanel::Right]
            .into_iter()
            .map(|side| self.panels.side(side))
            .filter(|panel| panel.source.is_local())
            .map(|panel| WatchTarget {
                dir: panel.current_path.clone(),
                prefer_poll: limit > 0 && panel.entries.len() > limit,
            })
            .collect()
    }

    /// Re-arms the monitors for the folders shown now and rereads the tabs
    /// whose folder changed. Called once per event-loop pass.
    pub fn poll_auto_refresh(&mut self, show_hidden: bool) {
        let targets = self.auto_refresh_targets();
        self.auto_refresh.sync(targets);
        let now = Instant::now();
        for change in self.auto_refresh.take_due(now) {
            self.apply_dir_change(change, show_hidden, now);
        }
    }

    /// Rereads every local tab showing `change.dir` (shown or not) through
    /// the regular listing path, which also updates Git badges, and measures
    /// the changed folders' sizes again. A tab still loading gets the change
    /// again later instead of restarting its listing.
    fn apply_dir_change(&mut self, change: DirChange, show_hidden: bool, now: Instant) {
        let mut ready: Vec<TabId> = Vec::new();
        let mut busy = false;
        for (_, tab) in self.panels.all_tabs_mut() {
            let panel = &mut tab.panel;
            if !panel.source.is_local() || panel.current_path != change.dir {
                continue;
            }
            if panel.listing.is_running() {
                busy = true;
                continue;
            }
            panel.dir_sizes.invalidate(&change.entries);
            ready.push(tab.id);
        }
        for id in ready {
            self.refresh_tab_quietly(id, show_hidden);
        }
        if busy {
            self.auto_refresh.defer(change, now);
        }
    }
}
