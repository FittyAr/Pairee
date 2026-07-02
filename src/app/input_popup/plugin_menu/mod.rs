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
    let popup_state = match state.active_popup.clone() {
        Some(PopupType::PluginMenu {
            active_tab,
            cursor_idx,
            installed,
            registry,
            search_query,
            is_searching,
            editing_query,
            dev_results,
            dev_wizard_step,
            dev_wizard_data,
            installed_loading,
            installed_loading_status,
            dev_loading,
            dev_loading_status,
            dev_loading_progress,
        }) => (
            active_tab,
            cursor_idx,
            installed,
            registry,
            search_query,
            is_searching,
            editing_query,
            dev_results,
            dev_wizard_step,
            dev_wizard_data,
            installed_loading,
            installed_loading_status,
            dev_loading,
            dev_loading_status,
            dev_loading_progress,
        ),
        _ => return Err(()),
    };

    let (
        mut active_tab,
        mut cursor_idx,
        mut installed,
        mut registry,
        mut search_query,
        is_searching,
        mut editing_query,
        mut dev_results,
        mut dev_wizard_step,
        mut dev_wizard_data,
        installed_loading,
        installed_loading_status,
        mut dev_loading,
        mut dev_loading_status,
        mut dev_loading_progress,
    ) = popup_state;

    // Handle global escape to close if not editing query
    if key.code == KeyCode::Esc {
        if (active_tab == 1 && editing_query) || (active_tab == 2 && editing_query) {
            editing_query = false;
            dev_wizard_step = 0;
            dev_wizard_data.clear();
            state.active_popup = Some(PopupType::PluginMenu {
                active_tab,
                cursor_idx,
                installed,
                registry,
                search_query,
                is_searching,
                editing_query,
                dev_results,
                dev_wizard_step,
                dev_wizard_data,
                installed_loading,
                installed_loading_status,
                dev_loading,
                dev_loading_status,
                dev_loading_progress,
            });
            return Ok(None);
        } else {
            state.active_popup = None;
            return Ok(None);
        }
    }

    match key.code {
        KeyCode::Tab => {
            let dev_mode = context.config.settings.plugins_developer_mode;
            if !(active_tab == 1 && editing_query) && !(active_tab == 2 && editing_query) {
                active_tab = if active_tab == 0 {
                    1
                } else if active_tab == 1 {
                    if dev_mode { 2 } else { 0 }
                } else {
                    0
                };
                cursor_idx =
                    if active_tab == 2 && context.config.settings.active_dev_plugin.is_none() {
                        1
                    } else {
                        0
                    };
                editing_query = false;
                dev_results = String::new();
                state.active_popup = Some(PopupType::PluginMenu {
                    active_tab,
                    cursor_idx,
                    installed,
                    registry,
                    search_query,
                    is_searching,
                    editing_query,
                    dev_results,
                    dev_wizard_step: 0,
                    dev_wizard_data: Vec::new(),
                    installed_loading,
                    installed_loading_status,
                    dev_loading,
                    dev_loading_status,
                    dev_loading_progress,
                });
                return Ok(None);
            }
        }
        _ => {}
    }

    let action = None;
    if active_tab == 0 {
        installed::handle_installed(key, context, &mut cursor_idx, &mut installed);
    } else if active_tab == 1 {
        search::handle_search(
            key,
            &mut cursor_idx,
            &mut registry,
            &mut search_query,
            &mut editing_query,
        );
    } else {
        let left_path = state.left_panel.current_path.clone();
        let right_path = state.right_panel.current_path.clone();
        dev::handle_dev(
            key,
            state,
            context,
            &left_path,
            &right_path,
            &mut cursor_idx,
            &mut installed,
            &mut search_query,
            &mut editing_query,
            &mut dev_results,
            &mut dev_wizard_step,
            &mut dev_wizard_data,
        );
        // Pull back the live loading fields from the popup state because
        // `handle_dev` may have flipped them (e.g. when starting a new op
        // or when a background update landed).
        if let Some(PopupType::PluginMenu {
            dev_loading: dl,
            dev_loading_status: dls,
            dev_loading_progress: dlp,
            ..
        }) = &state.active_popup
        {
            dev_loading = *dl;
            dev_loading_status = dls.clone();
            dev_loading_progress = *dlp;
        }
    }

    state.active_popup = Some(PopupType::PluginMenu {
        active_tab,
        cursor_idx,
        installed,
        registry,
        search_query,
        is_searching,
        editing_query,
        dev_results,
        dev_wizard_step,
        dev_wizard_data,
        installed_loading,
        installed_loading_status,
        dev_loading,
        dev_loading_status,
        dev_loading_progress,
    });

    Ok(action)
}
