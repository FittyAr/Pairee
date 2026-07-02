use super::reload_installed_plugins;
use crate::app::context::AppContext;
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_dev(
    key: KeyEvent,
    state: &mut crate::app::state::AppState,
    context: &mut AppContext,
    left_panel_path: &std::path::Path,
    right_panel_path: &std::path::Path,
    cursor_idx: &mut usize,
    installed: &mut Vec<(String, String, bool, bool, Option<String>)>,
    search_query: &mut String,
    editing_query: &mut bool,
    dev_results: &mut String,
    dev_wizard_step: &mut usize,
    dev_wizard_data: &mut Vec<String>,
) {
    // Runtime check: Verify that the active development plugin folder still exists
    let active_plugin = context.config.settings.active_dev_plugin.clone();
    if let Some(ref folder_name) = active_plugin {
        let plugins_dev_dir = &context.config.settings.plugins_dev_dir;
        let path = if std::path::Path::new(folder_name).is_absolute() {
            std::path::PathBuf::from(folder_name)
        } else {
            std::path::PathBuf::from(plugins_dev_dir).join(folder_name)
        };
        if !path.exists() || !path.is_dir() || !path.join("manifest.toml").exists() {
            context.config.settings.active_dev_plugin = None;
            let _ = context.config.save();
            *dev_results =
                "Stale active development plugin deselected (directory no longer exists)."
                    .to_string();
            *installed = reload_installed_plugins(context, &None);
        }
    }

    if *editing_query {
        match key.code {
            KeyCode::Backspace => {
                search_query.pop();
            }
            KeyCode::Char(c) => {
                search_query.push(c);
            }
            KeyCode::Enter => {
                if *dev_wizard_step == 1 {
                    let name = search_query.clone().trim().to_string();
                    if !name.is_empty() {
                        dev_wizard_data.push(name);
                        search_query.clear();
                        *dev_wizard_step = 2; // Prompt for description
                    }
                } else if *dev_wizard_step == 2 {
                    let desc = search_query.clone().trim().to_string();
                    dev_wizard_data.push(desc);
                    search_query.clear();
                    *dev_wizard_step = 3; // Prompt for author
                } else if *dev_wizard_step == 3 {
                    let author = search_query.clone().trim().to_string();
                    dev_wizard_data.push(author);
                    search_query.clear();
                    *editing_query = false;
                    *dev_wizard_step = 0;

                    let plugins_dev_dir = &context.config.settings.plugins_dev_dir;
                    let folder_name = if dev_wizard_data[0].ends_with(".pairee") {
                        dev_wizard_data[0].clone()
                    } else {
                        format!("{}.pairee", dev_wizard_data[0])
                    };
                    let target_path = std::path::PathBuf::from(plugins_dev_dir).join(&folder_name);
                    let _ = std::fs::create_dir_all(&target_path);
                    if let Ok(current_dir) = std::env::current_dir() {
                        if std::env::set_current_dir(plugins_dev_dir).is_ok() {
                            match crate::plugin::developer_tool::init(
                                &folder_name,
                                &dev_wizard_data[1],
                                &dev_wizard_data[2],
                                false,
                            ) {
                                Ok(_) => {
                                    let name_without_suffix =
                                        folder_name.strip_suffix(".pairee").unwrap_or(&folder_name);
                                    *dev_results = t("plugin_dev_init_ok")
                                        .replace("{}", name_without_suffix)
                                        .replace("{:?}", &target_path.to_string_lossy());
                                    context.config.settings.active_dev_plugin =
                                        Some(folder_name.clone());
                                    let _ = context.config.save();
                                }
                                Err(e) => {
                                    *dev_results =
                                        t("plugin_dev_init_err").replace("{:?}", &format!("{}", e));
                                }
                            }
                            let _ = std::env::set_current_dir(current_dir);
                        }
                    }
                    dev_wizard_data.clear();
                    *installed = reload_installed_plugins(context, &None);
                } else if *dev_wizard_step == 5 {
                    let commit_msg = search_query.clone().trim().to_string();
                    if !commit_msg.is_empty() {
                        dev_wizard_data.push(commit_msg);
                        search_query.clear();
                        *dev_wizard_step = 6; // Prompt for GitHub Token
                    }
                } else if *dev_wizard_step == 6 {
                    let token = search_query.clone().trim().to_string();
                    let plugin_path_str = dev_wizard_data[0].clone();
                    let commit_msg = dev_wizard_data[1].clone();
                    dev_wizard_data.clear();
                    *editing_query = false;
                    *dev_wizard_step = 0;
                    search_query.clear();

                    // 1. Commit locally first
                    let plugin_path = std::path::PathBuf::from(&plugin_path_str);
                    let manifest_path = plugin_path.join("manifest.toml");
                    let mut plugin_name = String::new();
                    if let Ok(manifest_content) = std::fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) =
                            crate::plugin::loader::PluginManifest::parse(&manifest_content)
                        {
                            plugin_name = manifest.name;
                        }
                    }

                    let mut local_err = None;
                    match crate::plugin::developer_tool::package_to_registry(&plugin_path) {
                        Ok(_) => {
                            if let Err(e) =
                                crate::plugin::developer_tool::commit_registry_changes(&commit_msg)
                            {
                                local_err = Some(
                                    t("plugin_dev_err_git_commit")
                                        .replace("{:?}", &format!("{:?}", e)),
                                );
                            }
                        }
                        Err(e) => {
                            local_err = Some(
                                t("plugin_dev_err_package_registry")
                                    .replace("{:?}", &format!("{:?}", e)),
                            );
                        }
                    }

                    if let Some(err) = local_err {
                        *dev_results = err;
                    } else {
                        let temp_dir = crate::config::paths::get_cache_dir().join("temp_registry");
                        if token.is_empty() {
                            *dev_results = t("plugin_dev_no_token_inst")
                                .replace("{}", &temp_dir.display().to_string());
                        } else {
                            let tx = crate::plugin::PluginManager::get_sender();
                            tokio::spawn(async move {
                                match crate::plugin::developer_tool::run_automatic_submit(
                                    &token,
                                    &commit_msg,
                                    &plugin_name,
                                )
                                .await
                                {
                                    Ok(msg) => {
                                        let _ = tx
                                            .send(crate::plugin::manager::PluginRequest::Notify {
                                                title: t("plugin_dev_toast_submitted_title"),
                                                msg,
                                                level: "info".to_string(),
                                            })
                                            .await;
                                    }
                                    Err(e) => {
                                        let _ = tx
                                            .send(crate::plugin::manager::PluginRequest::Notify {
                                                title: t("plugin_dev_toast_submit_fail_title"),
                                                msg: format!("{:?}", e),
                                                level: "error".to_string(),
                                            })
                                            .await;
                                    }
                                }
                            });
                            *dev_results = t("plugin_dev_fork_push_bg");
                        }
                    }
                    *installed = reload_installed_plugins(context, &None);
                }
            }
            _ => {}
        }
    } else {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                let has_active = context.config.settings.active_dev_plugin.is_some();
                if *cursor_idx == 0 {
                    *cursor_idx = 5;
                } else if has_active && *cursor_idx == 2 {
                    *cursor_idx = 0; // Skip 1 (Init) because it's disabled
                } else {
                    *cursor_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                let has_active = context.config.settings.active_dev_plugin.is_some();
                if *cursor_idx >= 5 {
                    *cursor_idx = 0;
                } else if has_active && *cursor_idx == 0 {
                    *cursor_idx = 2; // Skip 1 (Init) because it's disabled
                } else {
                    *cursor_idx += 1;
                }
            }
            KeyCode::Backspace | KeyCode::Delete | KeyCode::Char('d') | KeyCode::Char('D') => {
                if *cursor_idx == 0 && context.config.settings.active_dev_plugin.is_some() {
                    context.config.settings.active_dev_plugin = None;
                    let _ = context.config.save();
                    *dev_results = "Development plugin deselected.".to_string();
                    *installed = reload_installed_plugins(context, &None);
                }
            }
            KeyCode::Enter => {
                let plugins_dev_dir = &context.config.settings.plugins_dev_dir;
                let active_plugin = context.config.settings.active_dev_plugin.clone();
                match *cursor_idx {
                    0 => {
                        // Select Active Plugin - open selection list modal!
                        let mut options = Vec::new();
                        // Option to deselect/none
                        options.push(("[Deselect / None]".to_string(), "deselect".to_string()));

                        // 1. Scan plugins_dev_dir
                        if let Ok(entries) = std::fs::read_dir(plugins_dev_dir) {
                            for entry in entries.filter_map(Result::ok) {
                                let path = entry.path();
                                if path.is_dir() && path.join("manifest.toml").exists() {
                                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                        options.push((name.to_string(), name.to_string()));
                                    }
                                }
                            }
                        }

                        // 2. Scan Panel 1
                        let left_has_manifest = left_panel_path.join("manifest.toml").exists();
                        if left_has_manifest {
                            if let Some(name) = left_panel_path.file_name().and_then(|n| n.to_str())
                            {
                                options.push((
                                    format!("[Panel 1] {} ({})", name, left_panel_path.display()),
                                    left_panel_path.to_string_lossy().to_string(),
                                ));
                            }
                        }

                        // 3. Scan Panel 2
                        let right_has_manifest = right_panel_path.join("manifest.toml").exists();
                        if right_has_manifest {
                            if let Some(name) =
                                right_panel_path.file_name().and_then(|n| n.to_str())
                            {
                                options.push((
                                    format!("[Panel 2] {} ({})", name, right_panel_path.display()),
                                    right_panel_path.to_string_lossy().to_string(),
                                ));
                            }
                        }

                        // Transition to SelectDevPlugin popup
                        let previous_popup = Box::new(state.active_popup.clone().unwrap());
                        state.active_popup = Some(crate::app::state::PopupType::SelectDevPlugin {
                            options,
                            cursor_idx: 0,
                            previous_popup,
                        });
                    }
                    1 => {
                        // Init Plugin (disabled if active plugin is selected)
                        if active_plugin.is_some() {
                            *dev_results = t("plugin_dev_desc_init_disabled");
                        } else {
                            *editing_query = true;
                            *search_query = String::new();
                            *dev_results = String::new();
                            *dev_wizard_step = 1;
                            *dev_wizard_data = Vec::new();
                        }
                    }
                    2 => {
                        // Lint active plugin
                        if let Some(plugin_folder) = active_plugin {
                            let path = if std::path::Path::new(&plugin_folder).is_absolute() {
                                std::path::PathBuf::from(&plugin_folder)
                            } else {
                                std::path::PathBuf::from(plugins_dev_dir).join(&plugin_folder)
                            };
                            let mut report = String::new();
                            if path.exists() && path.is_dir() && path.join("manifest.toml").exists()
                            {
                                let name = plugin_folder
                                    .strip_suffix(".pairee")
                                    .unwrap_or(&plugin_folder)
                                    .to_string();
                                let manifest_path = path.join("manifest.toml");
                                let main_path = path.join("main.lua");

                                report.push_str(&t("plugin_dev_lint_start").replace("{}", &name));
                                let mut warnings = 0;
                                if !manifest_path.exists() {
                                    report.push_str(&t("plugin_dev_lint_err_manifest"));
                                    warnings += 1;
                                }
                                if !main_path.exists() {
                                    report.push_str(&t("plugin_dev_lint_err_lua"));
                                    warnings += 1;
                                }
                                if manifest_path.exists() && main_path.exists() {
                                    if let Ok(lua_code) = std::fs::read_to_string(&main_path) {
                                        let forbidden = [
                                            "os.execute",
                                            "io.open",
                                            "os.system",
                                            "dofile",
                                            "loadfile",
                                        ];
                                        for f in &forbidden {
                                            if lua_code.contains(f) {
                                                report.push_str(
                                                    &t("plugin_dev_lint_warn_unsafe")
                                                        .replace("{}", f),
                                                );
                                                warnings += 1;
                                            }
                                        }
                                    }
                                }
                                if warnings == 0 {
                                    report.push_str(&t("plugin_dev_lint_ok"));
                                } else {
                                    report.push_str(
                                        &t("plugin_dev_lint_warn_total")
                                            .replace("{}", &format!("{}", warnings)),
                                    );
                                }
                            } else {
                                report = format!(
                                    "Error: Plugin directory '{}' no longer exists.",
                                    plugin_folder
                                );
                            }
                            *dev_results = report;
                        } else {
                            *dev_results = t("plugin_dev_no_active_err");
                        }
                    }
                    3 => {
                        // Package active plugin
                        if let Some(plugin_folder) = active_plugin {
                            let path = if std::path::Path::new(&plugin_folder).is_absolute() {
                                std::path::PathBuf::from(&plugin_folder)
                            } else {
                                std::path::PathBuf::from(plugins_dev_dir).join(&plugin_folder)
                            };
                            let mut report = String::new();
                            if path.exists() && path.is_dir() && path.join("manifest.toml").exists()
                            {
                                report.push_str(&format!(
                                    "{} '{}'...\n",
                                    t("plugin_dev_pack_start").trim(),
                                    plugin_folder
                                ));
                                match crate::plugin::developer_tool::package_to_registry(&path) {
                                    Ok(msg) => {
                                        report.push_str(&format!("✓ {}\n", msg));
                                    }
                                    Err(e) => {
                                        report.push_str(&format!("✗ Failed: {:?}\n", e));
                                    }
                                }
                            } else {
                                report = format!(
                                    "Error: Plugin directory '{}' no longer exists.",
                                    plugin_folder
                                );
                            }
                            *dev_results = report;
                        } else {
                            *dev_results = t("plugin_dev_no_active_err");
                        }
                    }
                    4 => {
                        // Install active plugin locally
                        if let Some(plugin_folder) = active_plugin {
                            let path = if std::path::Path::new(&plugin_folder).is_absolute() {
                                std::path::PathBuf::from(&plugin_folder)
                            } else {
                                std::path::PathBuf::from(plugins_dev_dir).join(&plugin_folder)
                            };
                            let mut report = String::new();
                            let dest_base = crate::config::paths::get_config_dir().join("plugins");
                            let mut lock = crate::plugin::updater::read_lockfile();
                            if path.exists() && path.is_dir() && path.join("manifest.toml").exists()
                            {
                                let manifest_path = path.join("manifest.toml");
                                if let Ok(manifest_content) =
                                    std::fs::read_to_string(&manifest_path)
                                {
                                    if let Ok(manifest) =
                                        crate::plugin::loader::PluginManifest::parse(
                                            &manifest_content,
                                        )
                                    {
                                        let name = manifest.name.clone();
                                        let version = manifest.version.clone();
                                        let dest_dir = dest_base.join(format!("{}.pairee", name));
                                        let _ = std::fs::create_dir_all(&dest_dir);

                                        let mut copied_files = Vec::new();
                                        for (rel_path_str, src_file_path) in
                                            crate::plugin::loader::get_plugin_files(&path)
                                        {
                                            let dest_file_path = dest_dir.join(&rel_path_str);
                                            if let Some(parent) = dest_file_path.parent() {
                                                let _ = std::fs::create_dir_all(parent);
                                            }
                                            if std::fs::copy(&src_file_path, &dest_file_path)
                                                .is_ok()
                                            {
                                                copied_files.push(rel_path_str);
                                            }
                                        }

                                        let mut files_hash = std::collections::HashMap::new();
                                        for (rel, p) in
                                            crate::plugin::loader::get_plugin_files(&dest_dir)
                                        {
                                            if let Ok(h) =
                                                crate::update::downloader::compute_sha256(&p)
                                            {
                                                files_hash.insert(rel, h);
                                            }
                                        }
                                        lock.plugins.insert(
                                            name.clone(),
                                            crate::plugin::updater::PinnedPlugin {
                                                version,
                                                pinned: false,
                                                files: files_hash,
                                            },
                                        );

                                        report.push_str(&format!(
                                            "✓ Installed '{}' locally (copied {} file(s))\n",
                                            name,
                                            copied_files.len()
                                        ));
                                        let _ = crate::plugin::updater::write_lockfile(&lock);
                                        report.push_str(&format!(
                                            "\n{}",
                                            t("plugin_dev_local_sync_ok")
                                        ));
                                    } else {
                                        report =
                                            "Error: Failed to parse manifest.toml.".to_string();
                                    }
                                } else {
                                    report = "Error: Failed to read manifest.toml.".to_string();
                                }
                            } else {
                                report = format!(
                                    "Error: Plugin directory '{}' no longer exists.",
                                    plugin_folder
                                );
                            }
                            *dev_results = report;
                            *installed = reload_installed_plugins(context, &None);
                        } else {
                            *dev_results = t("plugin_dev_no_active_err");
                        }
                    }
                    5 => {
                        // Submit Plugin
                        if let Some(plugin_folder) = active_plugin {
                            let path = if std::path::Path::new(&plugin_folder).is_absolute() {
                                std::path::PathBuf::from(&plugin_folder)
                            } else {
                                std::path::PathBuf::from(plugins_dev_dir).join(&plugin_folder)
                            };
                            if path.exists() && path.is_dir() && path.join("manifest.toml").exists()
                            {
                                match crate::plugin::developer_tool::validate_for_publish(&path) {
                                    Ok(_) => {
                                        *editing_query = true;
                                        *search_query = String::new();
                                        *dev_results = String::new();
                                        *dev_wizard_step = 5;
                                        *dev_wizard_data = vec![path.to_string_lossy().to_string()];
                                    }
                                    Err(err_msg) => {
                                        *dev_results = err_msg;
                                    }
                                }
                            } else {
                                *dev_results = format!(
                                    "Error: Plugin directory '{}' no longer exists.",
                                    plugin_folder
                                );
                            }
                        } else {
                            *dev_results = t("plugin_dev_no_active_err");
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

pub fn handle_select_popup(
    state: &mut crate::app::state::AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<crate::keybindings::Action>, ()> {
    let (options, mut cursor_idx, previous_popup) = match state.active_popup.clone() {
        Some(crate::app::state::PopupType::SelectDevPlugin {
            options,
            cursor_idx,
            previous_popup,
        }) => (options, cursor_idx, previous_popup),
        _ => return Err(()),
    };

    match key.code {
        KeyCode::Esc => {
            state.active_popup = Some(*previous_popup);
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            if !options.is_empty() {
                if cursor_idx == 0 {
                    cursor_idx = options.len() - 1;
                } else {
                    cursor_idx -= 1;
                }
            }
            state.active_popup = Some(crate::app::state::PopupType::SelectDevPlugin {
                options,
                cursor_idx,
                previous_popup,
            });
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            if !options.is_empty() {
                if cursor_idx >= options.len() - 1 {
                    cursor_idx = 0;
                } else {
                    cursor_idx += 1;
                }
            }
            state.active_popup = Some(crate::app::state::PopupType::SelectDevPlugin {
                options,
                cursor_idx,
                previous_popup,
            });
        }
        KeyCode::Enter => {
            if cursor_idx < options.len() {
                let (_, value) = &options[cursor_idx];
                if value.is_empty() || value == "deselect" {
                    context.config.settings.active_dev_plugin = None;
                    let _ = context.config.save();
                } else {
                    context.config.settings.active_dev_plugin = Some(value.clone());
                    let _ = context.config.save();
                }
            }

            let mut prev = *previous_popup;
            if let crate::app::state::PopupType::PluginMenu {
                ref mut installed,
                ref mut dev_results,
                ..
            } = prev
            {
                *installed = reload_installed_plugins(context, &None);
                if let Some(ref active) = context.config.settings.active_dev_plugin {
                    *dev_results = format!("Selected active development plugin: {}", active);
                } else {
                    *dev_results = "Development plugin deselected.".to_string();
                }
            }
            state.active_popup = Some(prev);
        }
        _ => {}
    }

    Ok(None)
}
