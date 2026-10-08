//! Panel (re)loading. Listings, git status and free space are computed by a
//! background job per panel (`PanelState::listing`); results are applied on
//! the UI thread by [`AppState::poll_panel_listings`].

pub mod filter;
pub mod listing;

#[cfg(test)]
mod tests;

use super::{ActivePanel, AppState, PanelState};
use filter::{partition_entries_by_mask, skip_auto_update};
use listing::{ListingOptions, ListingRequest, PanelListing};

impl AppState {
    /// Digit runs compare numerically when "treat digits as numbers" is on or
    /// the sorting collation is set to `natural`.
    pub fn natural_sort(&self) -> bool {
        self.treat_digits_as_numbers || self.sorting_collation == "natural"
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

    /// Starts a background reread of one panel. A request superseded by a
    /// newer one for the same panel is cancelled and its result dropped.
    pub fn refresh_panel(&mut self, side: ActivePanel, show_hidden: bool, force: bool) {
        let options = self.listing_options(side, show_hidden);
        let limit = self.disable_panel_update_object_count;
        let panel = self.panels.side_mut(side);
        let path = panel.current_path.clone();
        let changed = path != panel.last_path;
        if changed {
            panel.quick_filter_mask = None;
            panel.last_path = path.clone();
            emit_on_cd(&path, side);
        }
        if skip_auto_update(limit, panel.entries.len(), changed || force) {
            return;
        }
        let request = ListingRequest {
            path,
            ssh: panel.ssh_conn.clone(),
            options,
            filter_mask: panel.filter_mask.clone(),
            quick_filter_mask: panel.quick_filter_mask.clone(),
            want_attrs: panel.view_mode.needs_attrs(),
        };
        panel
            .listing
            .start(move |ctx| listing::run(&request, &|| ctx.is_cancelled()));
        // Inline execution (no runtime) has the result ready right away.
        self.poll_panel_listings();
    }

    /// Applies finished listings. Returns `true` when a panel changed.
    pub fn poll_panel_listings(&mut self) -> bool {
        let mut changed = false;
        for side in [ActivePanel::Left, ActivePanel::Right] {
            let panel = self.panels.side_mut(side);
            if let Some(listing) = panel.listing.poll() {
                apply_listing(panel, listing);
                changed = true;
            }
            // Finished folder sizes, or progress ticks of a running batch.
            changed |= panel.dir_sizes.poll() || panel.dir_sizes.is_running();
        }
        if changed {
            self.mark_ui_dirty();
        }
        changed
    }

    fn listing_options(&self, side: ActivePanel, show_hidden: bool) -> ListingOptions {
        let panel = self.panels.side(side);
        ListingOptions {
            show_hidden,
            case_sensitive: self.case_sensitive_sort,
            natural: self.natural_sort(),
            req_admin: self.req_admin_reading,
            sort_field: panel.sort_field,
            sort_reverse: panel.sort_reverse,
            folder_by_ext: self.sort_folder_names_by_extension,
            show_dotdot: self.show_dotdot_in_root_folders,
        }
    }

    /// Dynamically applies / updates in-memory sorting and filtering on the active panel.
    pub fn update_panel_filter(&mut self, active_panel: ActivePanel, mask: Option<String>) {
        let natural = self.natural_sort();
        let case_sensitive = self.case_sensitive_sort;
        let folder_by_ext = self.sort_folder_names_by_extension;
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

/// Installs a finished listing, keeping the cursor and selection stable.
pub(crate) fn apply_listing(panel: &mut PanelState, listing: PanelListing) {
    if listing.path != panel.current_path {
        // The panel moved on; a newer request is (or will be) in flight.
        return;
    }
    let same_dir = panel.listed_path.as_ref() == Some(&listing.path);
    if !same_dir {
        panel.dir_sizes.clear();
    }
    match listing.entries {
        Ok(entries) => {
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
            panel.dir_sizes.retain_listed(&present);
        }
        Err(err) => {
            log::warn!("Listing {:?} failed: {}", listing.path, err);
            panel.pending_focus = None;
        }
    }
    panel.listed_path = Some(listing.path);
    match listing.git {
        Some(git) => {
            panel.git_branch = Some(git.branch);
            panel.git_statuses = git.statuses;
        }
        None => {
            panel.git_branch = None;
            panel.git_statuses.clear();
        }
    }
    panel.free_space = listing.free_space;
    panel.attrs = listing.attrs;
}
