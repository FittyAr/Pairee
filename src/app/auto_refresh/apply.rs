//! Auto-refresh on the application state: which folders to monitor and
//! how a coalesced change reaches the tabs showing that folder.

use super::WatchTarget;
use crate::app::state::{ActivePanel, AppState, TabId};
use crate::fs::watch::DirChange;
use std::time::Instant;

impl AppState {
    /// Folders shown by the active tab of each side on the local disk or an
    /// SFTP server (archive tabs are not monitored). Folders holding more
    /// objects than `disable_panel_update_object_count` (when set) are not
    /// refreshed automatically, as the setting says.
    pub fn auto_refresh_targets(&self) -> Vec<WatchTarget> {
        let limit = self.disable_panel_update_object_count as usize;
        [ActivePanel::Left, ActivePanel::Right]
            .into_iter()
            .map(|side| self.panels.side(side))
            .filter(|panel| limit == 0 || panel.entries.len() <= limit)
            .filter(|panel| panel.source.archive().is_none())
            .map(|panel| WatchTarget {
                dir: panel.current_path.clone(),
                remote: panel.source.ssh().cloned(),
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

    /// Rereads every tab showing `change.dir` on the same filesystem (shown or not) through
    /// the regular listing path, which also updates Git badges, and measures
    /// the changed folders' sizes again: the reported entries here (the
    /// watcher knows before the folder times are updated, which Windows does
    /// lazily in listings), any folder whose time changed in the listing
    /// (`DirSizes::sync_listing`, as for every reread). A tab still loading
    /// gets the change again later instead of restarting its listing.
    fn apply_dir_change(&mut self, change: DirChange, show_hidden: bool, now: Instant) {
        let mut ready: Vec<TabId> = Vec::new();
        let mut busy = false;
        for (_, tab) in self.panels.all_tabs_mut() {
            let panel = &mut tab.panel;
            let origin = super::origin_of(panel.source.ssh());
            let monitored = panel.source.archive().is_none() && origin == change.origin;
            if !monitored || panel.current_path != change.dir {
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
