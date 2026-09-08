use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub mod dev;
pub mod installed;
pub mod search;

pub fn reload_installed_plugins(
    context: &AppContext,
    index: &Option<crate::plugin::updater::RegistryIndex>,
) -> Vec<(String, String, bool, bool, Option<String>)> {
    let lock = crate::plugin::updater::read_lockfile();
    let mut installed = Vec::new();
    for (name, info) in &lock.plugins {
        let trusted = context
            .config
            .settings
            .plugins
            .get(name)
            .map(|p| p.trusted)
            .unwrap_or(false);

        let update_available = if let Some(idx) = index {
            if let Some(reg_plugin) = idx.plugins.get(name) {
                if reg_plugin.version != info.version {
                    Some(reg_plugin.version.clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        installed.push((
            name.clone(),
            info.version.clone(),
            info.pinned,
            trusted,
            update_available,
        ));
    }
    installed
}

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let mut menu = match state.dialogs.top().cloned() {
        Some(PopupType::PluginMenu(m)) => m,
        _ => return Err(()),
    };

    // Handle global escape to close if not editing query
    if key.code == KeyCode::Esc {
        if menu.editing_query && (menu.active_tab == 1 || menu.active_tab == 2) {
            // Esc from search mode: clear query and restore full list
            menu.editing_query = false;
            if menu.active_tab == 1 {
                menu.search_query.clear();
                menu.registry = menu.all_registry.clone();
                menu.cursor_idx = 0;
            }
            menu.dev_wizard_step = 0;
            menu.dev_wizard_data.clear();
            state.dialogs.replace(PopupType::PluginMenu(menu));
            return Ok(None);
        } else {
            state.dialogs.clear();
            return Ok(None);
        }
    }

    if key.code == KeyCode::Tab {
        let dev_mode = context.config.settings.plugins_developer_mode;
        if !(menu.active_tab == 2 && menu.editing_query) {
            menu.active_tab = if menu.active_tab == 0 {
                1
            } else if menu.active_tab == 1 {
                if dev_mode { 2 } else { 0 }
            } else {
                0
            };
            menu.cursor_idx =
                if menu.active_tab == 2 && context.config.settings.active_dev_plugin.is_none() {
                    1
                } else {
                    0
                };
            // Auto-enter edit mode when switching to the Search tab
            menu.editing_query = menu.active_tab == 1;
            menu.dev_results = String::new();
            menu.dev_wizard_step = 0;
            menu.dev_wizard_data.clear();
            state.dialogs.replace(PopupType::PluginMenu(menu));
            return Ok(None);
        }
    }

    let action = None;
    if menu.active_tab == 0 {
        installed::handle_installed(key, context, &mut menu.cursor_idx, &mut menu.installed);
    } else if menu.active_tab == 1 {
        search::handle_search(
            key,
            &mut menu.cursor_idx,
            &mut menu.registry,
            &menu.all_registry,
            &mut menu.search_query,
            &mut menu.editing_query,
        );
    } else {
        let left_path = state.panels.left.current_path.clone();
        let right_path = state.panels.right.current_path.clone();
        dev::handle_dev(
            key,
            state,
            context,
            &left_path,
            &right_path,
            &mut menu.cursor_idx,
            &mut menu.installed,
            &mut menu.search_query,
            &mut menu.editing_query,
            &mut menu.dev_results,
            &mut menu.dev_wizard_step,
            &mut menu.dev_wizard_data,
        );
        // Pull back the live loading fields from the popup state because
        // `handle_dev` may have flipped them (e.g. when starting a new op
        // or when a background update landed).
        if let Some(PopupType::PluginMenu(live)) = state.dialogs.top() {
            menu.dev_loading = live.dev_loading;
            menu.dev_loading_status = live.dev_loading_status.clone();
            menu.dev_loading_progress = live.dev_loading_progress;
        }
    }

    state.dialogs.replace(PopupType::PluginMenu(menu));

    Ok(action)
}
