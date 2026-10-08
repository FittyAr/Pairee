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
) -> Vec<crate::plugin::installed::InstalledPlugin> {
    crate::plugin::installed::installed_rows(
        &crate::plugin::updater::read_lockfile(),
        &context.config.settings.plugins,
        index.as_ref(),
    )
}

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::PluginMenu(menu)) = state.dialogs.top_mut() else {
        return Err(());
    };

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
        } else {
            state.dialogs.clear();
        }
        return Ok(None);
    }

    if key.code == KeyCode::Tab && !(menu.active_tab == 2 && menu.editing_query) {
        let settings = &context.config.settings;
        menu.active_tab = match menu.active_tab {
            0 => 1,
            1 if settings.plugins_developer_mode => 2,
            _ => 0,
        };
        menu.cursor_idx = usize::from(menu.active_tab == 2 && settings.active_dev_plugin.is_none());
        // Auto-enter edit mode when switching to the Search tab
        menu.editing_query = menu.active_tab == 1;
        menu.dev_results = String::new();
        menu.dev_wizard_step = 0;
        menu.dev_wizard_data.clear();
        return Ok(None);
    }

    match menu.active_tab {
        0 => installed::handle_installed(key, context, &mut menu.cursor_idx, &mut menu.installed),
        1 => search::handle_search(
            key,
            &mut menu.cursor_idx,
            &mut menu.registry,
            &menu.all_registry,
            &mut menu.search_query,
            &mut menu.editing_query,
        ),
        _ => handle_dev_tab(state, key, context),
    }
    Ok(None)
}

/// The developer tab may open dialogs and start jobs, so it works on a copy
/// of the menu that is put back afterwards.
fn handle_dev_tab(state: &mut AppState, key: KeyEvent, context: &mut AppContext) {
    let Some(PopupType::PluginMenu(mut menu)) = state.dialogs.top().cloned() else {
        return;
    };
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
    state.dialogs.replace(PopupType::PluginMenu(menu));
}
