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
            dev_results,
        }) => (
            active_tab,
            cursor_idx,
            installed,
            registry,
            search_query,
            is_searching,
            editing_query,
            dev_results,
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
    ) = popup_state;

    // Handle global escape to close if not editing query
    if key.code == KeyCode::Esc {
        if (active_tab == 1 && editing_query) || (active_tab == 2 && editing_query) {
            editing_query = false;
            state.active_popup = Some(PopupType::PluginMenu {
                active_tab,
                cursor_idx,
                installed,
                registry,
                search_query,
                is_searching,
                editing_query,
                dev_results,
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
                cursor_idx = 0;
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

                    installed = reload_installed_plugins(context, &None);
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if let Some((name, _, _, _, _)) = installed.get(cursor_idx) {
                    let mut lock = crate::plugin::updater::read_lockfile();
                    if let Some(p) = lock.plugins.get_mut(name) {
                        p.pinned = !p.pinned;
                    }
                    let _ = crate::plugin::updater::write_lockfile(&lock);

                    installed = reload_installed_plugins(context, &None);
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
                if let Some((name, _, _, _, _)) = installed.get(cursor_idx) {
                    let _ = crate::plugin::updater::remove(name);
                    installed = reload_installed_plugins(context, &None);
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
    } else if active_tab == 1 {
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
                    let index_res = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(crate::plugin::updater::fetch_index())
                    });
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
    } else {
        // Tab 2: Developer Tools
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
                    let plugins_dir = crate::config::paths::get_config_dir().join("plugins");
                    let target_path = plugins_dir.join(&search_query);
                    let _ = std::fs::create_dir_all(&target_path);
                    if let Ok(current_dir) = std::env::current_dir() {
                        if std::env::set_current_dir(&plugins_dir).is_ok() {
                            match crate::plugin::developer_tool::init(&search_query) {
                                Ok(_) => {
                                    dev_results = format!(
                                        "✓ New plugin '{}' initialized successfully.\n\nBoilerplate files created:\n  - manifest.toml\n  - main.lua\n  - lang/en.toml\n\nTarget directory:\n{:?}",
                                        search_query, target_path
                                    );
                                }
                                Err(e) => {
                                    dev_results = format!("Error initializing plugin: {:?}", e);
                                }
                            }
                            let _ = std::env::set_current_dir(current_dir);
                        }
                    }
                    installed = reload_installed_plugins(context, &None);
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    if cursor_idx == 0 {
                        cursor_idx = 3;
                    } else {
                        cursor_idx -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    if cursor_idx >= 3 {
                        cursor_idx = 0;
                    } else {
                        cursor_idx += 1;
                    }
                }
                KeyCode::Enter => {
                    let plugins_dir = crate::config::paths::get_config_dir().join("plugins");
                    match cursor_idx {
                        0 => {
                            // Init Plugin
                            editing_query = true;
                            search_query = String::new();
                            dev_results = String::new();
                        }
                        1 => {
                            // Lint selected plugin
                            if let Some((selected_name, _, _, _, _)) = installed.get(0) {
                                let plugin_path = plugins_dir.join(selected_name);
                                let manifest_path = plugin_path.join("manifest.toml");
                                let main_path = plugin_path.join("main.lua");
                                let mut report = format!("Linting plugin '{}'...\n\n", selected_name);
                                let mut warnings = 0;
                                if !manifest_path.exists() {
                                    report.push_str("  [Error] manifest.toml not found!\n");
                                    warnings += 1;
                                }
                                if !main_path.exists() {
                                    report.push_str("  [Error] main.lua not found!\n");
                                    warnings += 1;
                                }
                                if manifest_path.exists() && main_path.exists() {
                                    if let Ok(lua_code) = std::fs::read_to_string(&main_path) {
                                        let forbidden = ["os.execute", "io.open", "os.system", "dofile", "loadfile"];
                                        for f in &forbidden {
                                            if lua_code.contains(f) {
                                                report.push_str(&format!("  [Warning] Potentially unsafe method call detected: '{}'\n", f));
                                                warnings += 1;
                                            }
                                        }
                                    }
                                }
                                if warnings == 0 {
                                    report.push_str("\n✓ Lint completed successfully! No issues found.");
                                } else {
                                    report.push_str(&format!("\nLint completed with {} warning(s)/error(s).", warnings));
                                }
                                dev_results = report;
                            } else {
                                dev_results = "Error: No installed plugins found to lint.".to_string();
                            }
                        }
                        2 => {
                            // Package selected plugin
                            if let Some((selected_name, version, _, _, _)) = installed.get(0) {
                                let plugin_path = plugins_dir.join(selected_name);
                                let manifest_path = plugin_path.join("manifest.toml");
                                let main_path = plugin_path.join("main.lua");
                                let lang_path = plugin_path.join("lang/en.toml");
                                
                                let mut report = format!("Packaging plugin '{}'...\n\n", selected_name);
                                let mut files_hash = std::collections::HashMap::new();
                                let files_to_hash = [("manifest.toml", &manifest_path), ("main.lua", &main_path), ("lang/en.toml", &lang_path)];
                                for (rel, path) in &files_to_hash {
                                    if path.exists() {
                                        if let Ok(hash) = crate::update::downloader::compute_sha256(path) {
                                            files_hash.insert(rel.to_string(), hash);
                                        }
                                    }
                                }
                                report.push_str("Generated registry entry to append to registry/index.toml:\n\n");
                                report.push_str(&format!("[plugins.{}]\n", selected_name));
                                report.push_str(&format!("name = \"{}\"\n", selected_name));
                                report.push_str(&format!("version = \"{}\"\n", version));
                                report.push_str("files = {\n");
                                for (f, h) in files_hash {
                                    report.push_str(&format!("    \"{}\" = \"{}\",\n", f, h));
                                }
                                report.push_str("}\n");
                                dev_results = report;
                            } else {
                                dev_results = "Error: No installed plugins found to package.".to_string();
                            }
                        }
                        3 => {
                            // Submit instructions
                            dev_results = "GitHub Pull Request Submission:\n\nTo submit your packaged plugin to the official registry:\n\n1. Run the interactive submission wizard in your shell:\n   > pairee developer submit\n\n2. Provide your GitHub Personal Access Token.\n3. The wizard will fork the repository, push your files, and submit a PR automatically.".to_string();
                        }
                        _ => {}
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
        dev_results,
    });

    Ok(None)
}
