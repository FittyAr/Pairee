use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

fn reload_installed_plugins(
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
        }) => (
            active_tab,
            cursor_idx,
            installed,
            registry,
            search_query,
            is_searching,
            editing_query,
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
    ) = popup_state;

    // Handle global escape to close if not editing query
    if key.code == KeyCode::Esc {
        if active_tab == 1 && editing_query {
            editing_query = false;
            state.active_popup = Some(PopupType::PluginMenu {
                active_tab,
                cursor_idx,
                installed,
                registry,
                search_query,
                is_searching,
                editing_query,
            });
            return Ok(None);
        } else {
            state.active_popup = None;
            return Ok(None);
        }
    }

    match key.code {
        KeyCode::Tab => {
            if !(active_tab == 1 && editing_query) {
                active_tab = if active_tab == 0 { 1 } else { 0 };
                cursor_idx = 0;
                editing_query = false;
                state.active_popup = Some(PopupType::PluginMenu {
                    active_tab,
                    cursor_idx,
                    installed,
                    registry,
                    search_query,
                    is_searching,
                    editing_query,
                });
                return Ok(None);
            }
        }
        _ => {}
    }

    if active_tab == 0 {
        // Installed Tab Inputs
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if !installed.is_empty() {
                    if cursor_idx == 0 {
                        cursor_idx = installed.len() - 1;
                    } else {
                        cursor_idx -= 1;
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if !installed.is_empty() {
                    if cursor_idx + 1 >= installed.len() {
                        cursor_idx = 0;
                    } else {
                        cursor_idx += 1;
                    }
                }
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                if let Some((name, _, _, _, _)) = installed.get(cursor_idx) {
                    if let Ok(mut config) = crate::config::AppConfig::load_or_create() {
                        let plugin_conf = config.settings.plugins.entry(name.clone()).or_insert_with(|| {
                            crate::config::settings::PluginConfig {
                                name: name.clone(),
                                trusted: false,
                            }
                        });
                        plugin_conf.trusted = !plugin_conf.trusted;
                        let _ = config.save();
                    }

                    if let Ok(c) = crate::config::AppConfig::load_or_create() {
                        context.config = c;
                    }

                    let index = tokio::runtime::Handle::current().block_on(crate::plugin::updater::fetch_index()).ok();
                    installed = reload_installed_plugins(context, &index);
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if let Some((name, _, _, _, _)) = installed.get(cursor_idx) {
                    let mut lock = crate::plugin::updater::read_lockfile();
                    if let Some(p) = lock.plugins.get_mut(name) {
                        p.pinned = !p.pinned;
                    }
                    let _ = crate::plugin::updater::write_lockfile(&lock);

                    let index = tokio::runtime::Handle::current().block_on(crate::plugin::updater::fetch_index()).ok();
                    installed = reload_installed_plugins(context, &index);
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
                if let Some((name, _, _, _, _)) = installed.get(cursor_idx) {
                    let _ = crate::plugin::updater::remove(name);
                    let index = tokio::runtime::Handle::current().block_on(crate::plugin::updater::fetch_index()).ok();
                    installed = reload_installed_plugins(context, &index);
                    cursor_idx = cursor_idx.min(installed.len().saturating_sub(1));
                }
            }
            KeyCode::Char('u') => {
                if let Some((name, _, _, _, _)) = installed.get(cursor_idx) {
                    let name_clone = name.clone();
                    let tx = crate::plugin::PluginManager::get_sender();
                    tokio::spawn(async move {
                        match crate::plugin::updater::install(&name_clone, None).await {
                            Ok(_) => {
                                let _ = tx.send(crate::plugin::manager::PluginRequest::Notify {
                                    title: "Plugin Update".to_string(),
                                    msg: format!("Plugin '{}' updated successfully!", name_clone),
                                    level: "info".to_string(),
                                }).await;
                            }
                            Err(e) => {
                                let _ = tx.send(crate::plugin::manager::PluginRequest::Notify {
                                    title: "Plugin Update Failed".to_string(),
                                    msg: format!("Failed to update '{}': {:?}", name_clone, e),
                                    level: "error".to_string(),
                                }).await;
                            }
                        }
                    });
                }
            }
            KeyCode::Char('U') => {
                let tx = crate::plugin::PluginManager::get_sender();
                tokio::spawn(async move {
                    match crate::plugin::updater::update(None).await {
                        Ok(_) => {
                            let _ = tx.send(crate::plugin::manager::PluginRequest::Notify {
                                title: "Plugins Update".to_string(),
                                msg: "All plugins updated successfully!".to_string(),
                                level: "info".to_string(),
                            }).await;
                        }
                        Err(e) => {
                            let _ = tx.send(crate::plugin::manager::PluginRequest::Notify {
                                title: "Plugins Update Failed".to_string(),
                                msg: format!("Failed to update plugins: {:?}", e),
                                level: "error".to_string(),
                            }).await;
                        }
                    }
                });
            }
            _ => {}
        }
    } else {
        // Search Registry Tab
        if editing_query {
            match key.code {
                KeyCode::Backspace => {
                    search_query.pop();
                }
                KeyCode::Char(c) => {
                    search_query.push(c);
                }
                KeyCode::Enter => {
                    editing_query = false;
                    let index_res = tokio::runtime::Handle::current().block_on(crate::plugin::updater::fetch_index());
                    if let Ok(idx) = index_res {
                        registry.clear();
                        let q_lower = search_query.to_lowercase();
                        for (name, p) in &idx.plugins {
                            if name.to_lowercase().contains(&q_lower)
                                || p.description
                                    .as_ref()
                                    .map(|d| d.to_lowercase().contains(&q_lower))
                                    .unwrap_or(false)
                            {
                                registry.push((
                                    name.clone(),
                                    p.version.clone(),
                                    p.description.clone().unwrap_or_default(),
                                    p.author.clone().unwrap_or_default(),
                                ));
                            }
                        }
                    }
                    cursor_idx = 0;
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    if !registry.is_empty() {
                        if cursor_idx == 0 {
                            cursor_idx = registry.len() - 1;
                        } else {
                            cursor_idx -= 1;
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    if !registry.is_empty() {
                        if cursor_idx + 1 >= registry.len() {
                            cursor_idx = 0;
                        } else {
                            cursor_idx += 1;
                        }
                    }
                }
                KeyCode::Char('/') => {
                    editing_query = true;
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    if !registry.is_empty() {
                        if let Some((name, _, _, _)) = registry.get(cursor_idx) {
                            let name_clone = name.clone();
                            let tx = crate::plugin::PluginManager::get_sender();
                            tokio::spawn(async move {
                                match crate::plugin::updater::install(&name_clone, None).await {
                                    Ok(_) => {
                                        let _ = tx.send(crate::plugin::manager::PluginRequest::Notify {
                                            title: "Plugin Installed".to_string(),
                                            msg: format!("Plugin '{}' installed successfully!", name_clone),
                                            level: "info".to_string(),
                                        }).await;
                                    }
                                    Err(e) => {
                                        let _ = tx.send(crate::plugin::manager::PluginRequest::Notify {
                                            title: "Plugin Installation Failed".to_string(),
                                            msg: format!("Failed to install '{}': {:?}", name_clone, e),
                                            level: "error".to_string(),
                                        }).await;
                                    }
                                }
                            });
                        }
                    }
                }
                _ => {}
            }
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
    });

    Ok(None)
}
