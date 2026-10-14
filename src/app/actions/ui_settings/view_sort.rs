//! View mode and sorting actions for UI settings.

use crate::app::context::AppContext;
use crate::app::state::popup::SortModesDialog;
use crate::app::state::popup::YaziSortPopup;
use crate::app::state::popup::YaziViewPopup;
use crate::app::state::{AppState, PanelViewMode, PopupType};
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
    if mode.needs_attrs() && panel.attrs.is_empty() && panel.source.is_local() {
        let side = state.panels.active;
        state.refresh_panel(side, show_hidden, true);
    }
}

/// The one-key sort and view-mode pickers (yazi workflow `s` / `v`).
pub fn open_picker(state: &mut AppState, action: &Action) -> bool {
    let popup = match action {
        Action::SortMenu => PopupType::YaziSortPopup(YaziSortPopup),
        Action::ViewModeMenu => PopupType::YaziViewPopup(YaziViewPopup),
        _ => return false,
    };
    state.dialogs.replace(popup);
    true
}

/// `cycle_panel_view` / `cycle_sort`: one key steps through the modes.
pub fn cycle_action(state: &mut AppState, action: &Action, context: &AppContext) -> bool {
    let show_hidden = context.config.settings.show_hidden;
    match action {
        Action::CyclePanelView => {
            let next = state.get_active_panel().view_mode.next();
            set_active_view_mode(state, next, show_hidden);
        }
        Action::CycleSort => {
            let panel = state.get_active_panel_mut();
            panel.sort_field = panel.sort_field.next();
            state.refresh_both_panels(show_hidden);
        }
        _ => return false,
    }
    true
}

pub fn handle_view_sort_action(
    state: &mut AppState,
    action: &Action,
    context: &mut AppContext,
) -> bool {
    if let Some(field) = crate::app::state::SortField::for_action(action) {
        state.get_active_panel_mut().sort_field = field;
        state.refresh_panel(
            state.panels.active,
            context.config.settings.show_hidden,
            true,
        );
        return true;
    }
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
            state.dialogs.replace(SortModesDialog {
                current,
                reverse,
                cursor_idx: 0,
            });
            true
        }
        // Only the active panel's order changes: the other one (maybe on a
        // server) is not read again.
        Action::ToggleSortReverse => {
            let panel = state.get_active_panel_mut();
            panel.sort_reverse = !panel.sort_reverse;
            state.refresh_panel(
                state.panels.active,
                context.config.settings.show_hidden,
                true,
            );
            true
        }
        Action::CompareFolder => {
            crate::app::sync::start_compare(state, context);
            true
        }
        Action::SyncDirs => {
            crate::app::sync::open_dialog(state, context);
            true
        }
        _ => false,
    }
}
