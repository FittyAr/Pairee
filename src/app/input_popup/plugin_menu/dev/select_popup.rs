//! Key-event handler for the "Select active development plugin" modal.

use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle_select_popup(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::SelectDevPlugin {
        options,
        cursor_idx,
        ..
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    let chosen = match list_key(ListKeys::ARROWS_VIM, key.code, cursor_idx, options.len()) {
        ListKey::Moved | ListKey::Other => return Ok(None),
        ListKey::Close => None,
        ListKey::Activate(idx) => Some(options.get(idx).map(|(_, value)| value.clone())),
    };
    let Some(PopupType::SelectDevPlugin { previous_popup, .. }) = state.dialogs.pop() else {
        return Ok(None);
    };
    let mut previous = *previous_popup;
    // Enter on a row selects (or deselects) the active development plugin.
    if let Some(choice) = chosen {
        if let Some(value) = choice {
            let active = (!value.is_empty() && value != "deselect").then_some(value);
            context.config.settings.active_dev_plugin = active;
            context.config.save_logging();
        }
        if let PopupType::PluginMenu(ref mut menu) = previous {
            menu.installed = super::reload_installed_plugins(context, &None);
            menu.dev_results = match &context.config.settings.active_dev_plugin {
                Some(active) => t("plugin_dev_selected").replace("{}", active),
                None => t("plugin_dev_deselected"),
            };
        }
    }
    state.dialogs.replace(previous);
    Ok(None)
}
