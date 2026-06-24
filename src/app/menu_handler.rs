use super::sys_helpers::get_system_drives;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;

/// Resolves the menu item from get_menu_items dynamically by index,
/// maps the active panel if needed, and returns the assigned action.
pub fn trigger_menu_item(
    state: &mut AppState,
    context: &mut AppContext,
    menu_idx: usize,
    item_idx: usize,
) -> Option<Action> {
    let items = crate::ui::menu::get_menu_items(
        menu_idx,
        state,
        &context.resolver,
        &context.config.settings,
    );

    let item = items.get(item_idx)?;

    if item.is_separator {
        return None;
    }

    if menu_idx == 0 || menu_idx == 4 {
        state.active_panel = if menu_idx == 4 {
            crate::app::state::ActivePanel::Right
        } else {
            crate::app::state::ActivePanel::Left
        };
    }

    if item.label == crate::config::localization::t("menu_hotplug_devices") {
        let drives = get_system_drives();
        state.active_popup = Some(PopupType::DriveSelect {
            panel: state.active_panel,
            drives,
            cursor_idx: 0,
        });
        return None;
    }

    item.action
}
