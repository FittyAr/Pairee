//! Panel (re)loading. Listings, git status and free space are computed by a
//! background job per tab (`PanelState::listing`); results are applied on
//! the UI thread by [`AppState::poll_panel_listings`].

pub mod filter;
pub mod listing;

#[cfg(test)]
mod tests;

use super::tabs::TabId;
use super::{ActivePanel, AppState, PanelState};
use crate::fs::archive::split_archive_path;
use filter::{partition_entries_by_mask, skip_auto_update};
use listing::{ListingOptions, ListingRequest, PanelListing};
use std::path::PathBuf;

impl AppState {
    /// Digit runs compare numerically when "treat digits as numbers" is on or
    /// the sorting collation is set to `natural`.
    pub fn natural_sort(&self) -> bool {
        self.prefs.sort.natural()
    }

    /// Rereads both panels in the background.
    ///
    /// Automatic rereads of an unchanged, very large directory are skipped
    /// when `disable_panel_update_object_count` is set; a panel that moved to
    /// another directory is always loaded.
    pub fn refresh_both_panels(&mut self, show_hidden: bool) {
        self.refresh_panel(ActivePanel::Left, show_hidden, false);
        self.refresh_panel(ActivePanel::Right, show_hidden, false);
    }

    /// Explicit reread (Ctrl+R / reread panel): ignores the object-count limit.
    pub fn force_refresh_both_panels(&mut self, show_hidden: bool) {
        self.refresh_panel(ActivePanel::Left, show_hidden, true);
        self.refresh_panel(ActivePanel::Right, show_hidden, true);
    }

    /// Rereads only the focused panel (e.g. after entering a directory).
    pub fn refresh_active_panel(&mut self, show_hidden: bool) {
        self.refresh_panel(self.panels.active, show_hidden, false);
    }

    /// Starts a background reread of the panel shown on `side`.
    pub fn refresh_panel(&mut self, side: ActivePanel, show_hidden: bool, force: bool) {
        let id = self.panels.active_tab_id(side);
        self.refresh_tab(id, show_hidden, force);
    }

    /// Starts a background reread of tab `id`, shown or not. A request
    /// superseded by a newer one for the same tab is cancelled and its
    /// result dropped; results always land in the tab that asked.
    pub fn refresh_tab(&mut self, id: TabId, show_hidden: bool, force: bool) {
        self.start_refresh(id, show_hidden, force, false);
    }

    /// Automatic reread of tab `id` after its folder changed on disk:
    /// always runs (the change is real) and shows no "Loading…" title.
    pub fn refresh_tab_quietly(&mut self, id: TabId, show_hidden: bool) {
        self.start_refresh(id, show_hidden, true, true);
    }

    fn start_refresh(&mut self, id: TabId, show_hidden: bool, force: bool, quiet: bool) {
        if self.divert_locked_tab(id, show_hidden) {
            return;
        }
        let Some((_, tab)) = self.panels.find_tab(id) else {
            return;
        };
        let options = self.listing_options(&tab.panel, show_hidden);
        let limit = self.prefs.disable_panel_update_object_count;
        let Some((side, tab)) = self.panels.find_tab_mut(id) else {
            return;
        };
        let panel = &mut tab.panel;
        let path = panel.current_path.clone();
        // Entering or leaving an archive switches the panel's source.
        panel.source = panel.source.locate(&path);
        let new_history = panel
            .nav
            .on_connection(panel.source.remote().map(|conn| conn.id()));
        let changed = path != panel.last_path;
        if changed {
            panel.quick_filter_mask = None;
            panel.visual = None;
            // The branch view belongs to the folder it was opened on.
            panel.branch_view = false;
            let left = std::mem::replace(&mut panel.last_path, path.clone());
            if !new_history {
                panel.nav.visited(left, &path);
            }
            emit_on_cd(&path, side);
            if panel.source.is_local() {
                self.frecency.visit(&path, crate::config::frecency::now());
            }
        }
        if skip_auto_update(limit, panel.entries.len(), changed || force) {
            return;
        }
        let request = ListingRequest {
            source: panel.source.clone(),
            options,
            filter_mask: panel.filter_mask.clone(),
            quick_filter_mask: panel.quick_filter_mask.clone(),
            want_attrs: panel.view_mode.needs_attrs(),
            branch: panel.branch_view,
            fallback: panel
                .restore_check
                .as_ref()
                .is_some_and(|c| c.covers(&path)),
            path,
        };
        panel.quiet_listing = quiet;
        panel.listing.start(move |ctx| {
            listing::run(&request, &|| ctx.is_cancelled(), &|found| ctx.report(found))
        });
        // Inline execution (no runtime) has the result ready right away.
        self.poll_panel_listings();
    }

    /// Applies finished listings of every tab, shown or not. Returns
    /// `true` when a panel changed.
    pub fn poll_panel_listings(&mut self) -> bool {
        let mut changed = false;
        let mut failed = Vec::new();
        for (_, tab) in self.panels.all_tabs_mut() {
            let panel = &mut tab.panel;
            if let Some(listing) = panel.listing.poll() {
                let show_hidden = listing.show_hidden;
                settle_restore_check(panel, &listing);
                if let Some(failure) = apply_listing(panel, listing) {
                    failed.push((tab.id, failure, show_hidden));
                }
                changed = true;
            }
            // Finished folder sizes, or progress ticks of a running batch.
            changed |= panel.dir_sizes.poll() || panel.dir_sizes.is_running();
            // Files found so far by a branch view being read.
            changed |= panel.branch_view && panel.listing.is_running();
            // A restored folder still being reached: the title may turn
            // from "connecting" to "unavailable".
            changed |= panel.restore_check.is_some() && panel.listing.is_running();
        }
        for (id, failure, show_hidden) in failed {
            match failure.not_entered {
                Some(path) => self.report_not_entered(&path, &failure.error),
                None => self.leave_unreadable_archive(id, &failure.error, show_hidden),
            }
        }
        if changed {
            self.mark_ui_dirty();
        }
        changed
    }

    /// Says why `path` (a folder or an archive) could not be opened.
    fn report_not_entered(&mut self, path: &std::path::Path, error: &str) {
        let key = match split_archive_path(path) {
            Some(_) => "archive_open_failed",
            None => "folder_open_failed",
        };
        self.dialogs.replace(super::PopupType::error(
            crate::config::localization::t(key)
                .replacen("{}", &path.to_string_lossy(), 1)
                .replacen("{}", error, 1),
        ));
    }

    /// An archive that cannot be read is not entered: the panel goes back
    /// to the folder holding it and the reason is shown.
    fn leave_unreadable_archive(&mut self, id: TabId, error: &str, show_hidden: bool) {
        let Some((_, tab)) = self.panels.find_tab_mut(id) else {
            return;
        };
        let panel = &mut tab.panel;
        let Some(root) = panel.source.archive().map(|a| a.root().to_path_buf()) else {
            return;
        };
        if panel.current_path != root {
            return;
        }
        let Some(parent) = root.parent() else {
            return;
        };
        panel.pending_focus = Some(crate::fs::file_name_lossy(&root));
        panel.current_path = parent.to_path_buf();
        self.dialogs.replace(super::PopupType::error(
            crate::config::localization::t("archive_open_failed")
                .replacen("{}", &root.to_string_lossy(), 1)
                .replacen("{}", error, 1),
        ));
        self.refresh_tab(id, show_hidden, true);
    }

    fn listing_options(&self, panel: &PanelState, show_hidden: bool) -> ListingOptions {
        ListingOptions {
            show_hidden,
            case_sensitive: self.prefs.sort.case_sensitive,
            natural: self.natural_sort(),
            req_admin: self.prefs.req_admin_reading,
            sort_field: panel.sort_field,
            sort_reverse: panel.sort_reverse,
            folder_by_ext: self.prefs.sort.folder_names_by_extension,
            show_dotdot: self.prefs.show_dotdot_in_root_folders,
        }
    }

    /// Dynamically applies / updates in-memory sorting and filtering on the active panel.
    pub fn update_panel_filter(&mut self, active_panel: ActivePanel, mask: Option<String>) {
        let natural = self.natural_sort();
        let case_sensitive = self.prefs.sort.case_sensitive;
        let folder_by_ext = self.prefs.sort.folder_names_by_extension;
        let panel = self.panels.side_mut(active_panel);

        let prev_selected_path = panel
            .entries
            .get(panel.cursor_index)
            .map(|e| e.path.clone());
        panel.quick_filter_mask = mask.filter(|m| !m.is_empty());

        crate::fs::list::sort_entries(
            &mut panel.entries,
            panel.sort_field,
            panel.sort_reverse,
            case_sensitive,
            natural,
            folder_by_ext,
        );
        if let Some(m) = panel.quick_filter_mask.as_deref() {
            panel.entries = partition_entries_by_mask(std::mem::take(&mut panel.entries), m);
        }

        panel.cursor_index = prev_selected_path
            .and_then(|path| panel.entries.iter().position(|e| e.path == path))
            .unwrap_or(0);
    }
}

/// Fires the plugin `on_cd` hook (no-op outside a Tokio runtime).
fn emit_on_cd(path: &std::path::Path, side: ActivePanel) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let payload = serde_json::json!({
        "path": path.to_string_lossy(),
        "side": match side {
            ActivePanel::Left => "left",
            ActivePanel::Right => "right",
        },
    });
    handle.spawn(async move {
        crate::plugin::hooks::emit_event("on_cd", payload).await;
    });
}

/// A listing that failed.
#[derive(Debug)]
pub(crate) struct ListingFailure {
    pub error: String,
    /// The folder that could not be entered; the panel stayed on the one
    /// it showed (`None`: a reread of the shown folder failed).
    pub not_entered: Option<PathBuf>,
}

/// Installs a finished listing, keeping the cursor and selection stable.
/// A folder that cannot be read is not entered: the panel stays on the
/// folder its entries belong to.
/// A listing that answers the panel's restore check ends it, unless the
/// folder (and every fallback) could not be read.
fn settle_restore_check(panel: &mut PanelState, listing: &PanelListing) {
    let asked = listing.fell_back_from.as_ref().unwrap_or(&listing.path);
    let Some(check) = panel.restore_check.as_mut() else {
        return;
    };
    if check.answered_by(asked, listing.entries.is_err()) && !check.failed() {
        panel.restore_check = None;
    }
}

pub(crate) fn apply_listing(
    panel: &mut PanelState,
    listing: PanelListing,
) -> Option<ListingFailure> {
    if listing.fell_back_from.as_ref() == Some(&panel.current_path) {
        // A restored folder that is gone: show where the job fell back.
        panel.current_path = listing.path.clone();
        panel.last_path = listing.path.clone();
        panel.source = crate::fs::vfs::PanelSource::Local;
        panel.cursor_index = 0;
        panel.pending_focus = None;
    }
    if listing.path != panel.current_path {
        // The panel moved on; a newer request is (or will be) in flight.
        return None;
    }
    let mut failure = None;
    let same_dir = panel.listed_path.as_ref() == Some(&listing.path);
    match listing.entries {
        Err(error) if !same_dir && panel.listed_path.is_some() => {
            log::warn!("Cannot enter {:?}: {}", listing.path, error);
            let shown = panel.listed_path.clone().unwrap_or_default();
            panel.pending_focus = None;
            panel.source = panel.source.locate(&shown);
            panel.current_path = shown.clone();
            panel.last_path = shown;
            if let Some(idx) = panel.entries.iter().position(|e| e.path == listing.path) {
                panel.cursor_index = idx;
            }
            return Some(ListingFailure {
                error,
                not_entered: Some(listing.path),
            });
        }
        Ok(entries) => {
            if !same_dir {
                panel.dir_sizes.clear();
            }
            let focus_path = panel
                .entries
                .get(panel.cursor_index)
                .filter(|_| same_dir)
                .map(|e| e.path.clone());
            panel.entries = entries;
            let focus_name = panel.pending_focus.take();
            let found = match (focus_name, focus_path) {
                (Some(name), _) => panel.entries.iter().position(|e| e.name == name),
                (None, Some(path)) => panel.entries.iter().position(|e| e.path == path),
                (None, None) => None,
            };
            if let Some(idx) = found {
                panel.cursor_index = idx;
            }
            if panel.cursor_index >= panel.entries.len() {
                panel.cursor_index = panel.entries.len().saturating_sub(1);
            }
            let present: std::collections::HashSet<_> =
                panel.entries.iter().map(|e| e.path.clone()).collect();
            panel.selected_paths.retain(|p| present.contains(p));
            panel.selection_order.retain(|p| present.contains(p));
            panel.dir_sizes.sync_listing(&panel.entries);
        }
        Err(error) => {
            log::warn!("Listing {:?} failed: {}", listing.path, error);
            panel.pending_focus = None;
            failure = Some(ListingFailure {
                error,
                not_entered: None,
            });
        }
    }
    panel.listed_path = Some(listing.path);
    match listing.git {
        Some(git) => {
            // The header then reads "[git: main, status unavailable]".
            panel.git_branch = Some(if git.status_failed {
                crate::config::localization::t("git_branch_status_failed")
                    .replace("{}", &git.branch)
            } else {
                git.branch
            });
            panel.git_statuses = git.statuses;
        }
        None => {
            panel.git_branch = None;
            panel.git_statuses.clear();
        }
    }
    panel.free_space = listing.free_space;
    panel.attrs = listing.attrs;
    failure
}
