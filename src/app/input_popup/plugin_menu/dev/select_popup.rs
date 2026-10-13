//! Key-event handler for the "Select active development plugin" modal.

use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::AppState;
use crate::app::state::popup::{
    Dialog, DialogScroll, KeyHandler, KeyOutcome, PluginMenuState, SelectDevPlugin,
};
use crate::config::localization::t;
use crate::ui::scrollbar::ScrollTargetId;
use crossterm::event::KeyEvent;

pub fn handle_select_popup(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> KeyOutcome {
    let Some(select) = state.dialogs.top_mut_as::<SelectDevPlugin>() else {
        return Err(());
    };
    let rows = select.options.len();
    let chosen = match list_key(ListKeys::ARROWS_VIM, key.code, &mut select.cursor_idx, rows) {
        ListKey::Moved | ListKey::Other => return Ok(None),
        ListKey::Close => None,
        ListKey::Activate(idx) => Some(select.options.get(idx).map(|(_, value)| value.clone())),
    };
    // Back to the dialog underneath (the Plugin Manager).
    state.dialogs.pop();
    // Enter on a row selects (or deselects) the active development plugin.
    if let Some(choice) = chosen {
        if let Some(value) = choice {
            let active = (!value.is_empty() && value != "deselect").then_some(value);
            context.config.settings.active_dev_plugin = active;
            context.config.save_logging();
        }
        if let Some(menu) = state.dialogs.top_mut_as::<PluginMenuState>() {
            menu.installed = super::reload_installed_plugins(context, &None);
            menu.dev_results = match &context.config.settings.active_dev_plugin {
                Some(active) => t("plugin_dev_selected").replace("{}", active),
                None => t("plugin_dev_deselected"),
            };
        }
    }
    Ok(None)
}

impl Dialog for SelectDevPlugin {
    fn key_handler(&self) -> KeyHandler {
        handle_select_popup
    }
    fn closes_on_outside_click(&self) -> bool {
        true
    }
    fn scroll_mut(&mut self, id: ScrollTargetId) -> Option<DialogScroll<'_>> {
        let len = self.options.len();
        (id == ScrollTargetId::PluginSelect)
            .then_some(DialogScroll::Cursor(&mut self.cursor_idx, len))
    }
}
