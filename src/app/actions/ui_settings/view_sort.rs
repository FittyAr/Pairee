//! View mode and sorting actions for UI settings.

use crate::app::context::AppContext;
use crate::app::state::{AppState, PanelViewMode, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;

/// Maps a `PanelView*` action to its view mode.
fn view_mode_for(action: &Action) -> Option<PanelViewMode> {
    Some(match action {
        Action::PanelViewBrief => PanelViewMode::Brief,
        Action::PanelViewMedium => PanelViewMode::Medium,
        Action::PanelViewFull => PanelViewMode::Full,
        Action::PanelViewWide => PanelViewMode::Wide,
        Action::PanelViewDetailed => PanelViewMode::Detailed,
        Action::PanelViewDescriptions => PanelViewMode::Descriptions,
        Action::PanelViewFileOwners => PanelViewMode::FileOwners,
        Action::PanelViewFileLinks => PanelViewMode::FileLinks,
        Action::PanelViewAltFull => PanelViewMode::AltFull,
        _ => return None,
    })
}

/// Switches the active panel's view mode; modes showing owners/permissions/
/// links trigger a background reread so their attributes are loaded.
fn set_active_view_mode(state: &mut AppState, mode: PanelViewMode, show_hidden: bool) {
    let panel = state.get_active_panel_mut();
    panel.view_mode = mode;
    if mode.needs_attrs() && panel.attrs.is_empty() && panel.ssh_conn.is_none() {
        let side = state.panels.active;
        state.refresh_panel(side, show_hidden, true);
    }
}

pub fn handle_view_sort_action(
    state: &mut AppState,
    action: &Action,
    context: &mut AppContext,
) -> bool {
    match action {
        Action::PanelViewBrief
        | Action::PanelViewMedium
        | Action::PanelViewFull
        | Action::PanelViewWide
        | Action::PanelViewDetailed
        | Action::PanelViewDescriptions
        | Action::PanelViewFileOwners
        | Action::PanelViewFileLinks
        | Action::PanelViewAltFull => {
            if let Some(mode) = view_mode_for(action) {
                set_active_view_mode(state, mode, context.config.settings.show_hidden);
            }
            true
        }
        Action::TogglePanelLeft => {
            state.panels.left_visible = !state.panels.left_visible;
            true
        }
        Action::TogglePanelRight => {
            state.panels.right_visible = !state.panels.right_visible;
            true
        }
        Action::ToggleBothPanels => {
            state.panels.both_hidden = !state.panels.both_hidden;
            true
        }
        Action::ToggleLongNames => {
            let panel = state.get_active_panel_mut();
            panel.show_long_names = !panel.show_long_names;
            true
        }
        Action::QuickView => {
            state.panels.quick_view_active = !state.panels.quick_view_active;
            if !state.panels.quick_view_active {
                if let Some(PopupType::QuickViewPanel(_)) = state.dialogs.top() {
                    state.dialogs.clear();
                }
            } else {
                state.update_quick_view_images(context.config.settings.image_preview_enabled);
            }
            true
        }
        Action::SortModes => {
            let current = state.get_active_panel().sort_field;
            let reverse = state.get_active_panel().sort_reverse;
            state.dialogs.replace(PopupType::SortModesDialog {
                current,
                reverse,
                cursor_idx: 0,
            });
            true
        }
        Action::ToggleSortReverse => {
            let current = state.get_active_panel().sort_reverse;
            state.get_active_panel_mut().sort_reverse = !current;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::SortByName => {
            state.get_active_panel_mut().sort_field = crate::app::state::SortField::Name;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::SortByExtension => {
            state.get_active_panel_mut().sort_field = crate::app::state::SortField::Extension;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::SortByWriteTime | Action::SortByCreationTime | Action::SortByAccessTime => {
            state.get_active_panel_mut().sort_field = crate::app::state::SortField::Date;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::SortBySize => {
            state.get_active_panel_mut().sort_field = crate::app::state::SortField::Size;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::SortUnsorted => {
            state.get_active_panel_mut().sort_field = crate::app::state::SortField::Unsorted;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::SortByDescription | Action::SortByOwner => {
            state.get_active_panel_mut().sort_field = crate::app::state::SortField::Name;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::CompareFolder => {
            if state.panels.left.ssh_conn.is_some() || state.panels.right.ssh_conn.is_some() {
                state
                    .dialogs
                    .replace(PopupType::Error(t("compare_local_only")));
                return true;
            }
            let left = state.panels.left.current_path.clone();
            let right = state.panels.right.current_path.clone();
            let options =
                crate::fs::compare::CompareOptions::from_settings(&context.config.settings);
            match crate::fs::compare_directories(&left, &right, &options) {
                Ok(diff) => {
                    for entry in &diff {
                        if entry.status != crate::fs::CompareStatus::Equal
                            && let Some(e) = state
                                .panels
                                .left
                                .entries
                                .iter()
                                .find(|e| e.name == entry.name)
                            && state.panels.left.selected_paths.insert(e.path.clone())
                        {
                            state.panels.left.selection_order.push(e.path.clone());
                        }
                    }
                    state.dialogs.replace(PopupType::CompareFoldersResult {
                        diff,
                        cursor_idx: 0,
                    });
                }
                Err(e) => {
                    state.dialogs.replace(PopupType::Error(
                        t("error_compare_failed").replace("{}", &e.to_string()),
                    ));
                }
            }
            true
        }
        _ => false,
    }
}
