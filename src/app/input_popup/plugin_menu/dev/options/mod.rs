//! Developer Tools key-event handler.

mod nav;
mod wizard;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PluginMenuState};
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};
use std::path::Path;

/// Keys of the developer tab; `panels` are the left and right panel paths.
pub fn handle_dev(
    key: KeyEvent,
    state: &mut AppState,
    context: &mut AppContext,
    panels: (&Path, &Path),
    menu: &mut PluginMenuState,
) {
    if let Some(folder_name) = context.config.settings.active_dev_plugin.clone() {
        let plugins_dev_dir = &context.config.settings.plugins_dev_dir;
        let path = if Path::new(&folder_name).is_absolute() {
            std::path::PathBuf::from(&folder_name)
        } else {
            std::path::PathBuf::from(plugins_dev_dir).join(&folder_name)
        };
        if !path.exists() || !path.is_dir() || !path.join("manifest.toml").exists() {
            context.config.settings.active_dev_plugin = None;
            context.config.save_logging();
            menu.dev_results = t("plugin_dev_stale_deselected");
            menu.installed = super::reload_installed_plugins(context, &None);
        }
    }

    if !menu.editing_query {
        nav::handle_navigation_or_enter(key, state, context, menu, panels);
        return;
    }
    // Typing a wizard answer.
    match key.code {
        KeyCode::Backspace => {
            menu.search_query.pop();
        }
        KeyCode::Char(c) => menu.search_query.push(c),
        KeyCode::Enter => wizard::handle_wizard_enter(state, context, menu),
        _ => {}
    }
}
