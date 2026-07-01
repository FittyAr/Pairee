use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
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
            dev_wizard_step,
            dev_wizard_data,
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
                    dev_wizard_step: 0,
                    dev_wizard_data: Vec::new(),
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
                        let plugin_conf = config
                            .settings
                            .plugins
                            .entry(name.clone())
                            .or_insert_with(|| crate::config::settings::PluginConfig {
                                name: name.clone(),
                                trusted: false,
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
                                let _ = tx
                                    .send(crate::plugin::manager::PluginRequest::Notify {
                                        title: t("plugin_toast_update_title"),
                                        msg: t("plugin_toast_update_ok").replace("{}", &name_clone),
                                        level: "info".to_string(),
                                    })
                                    .await;
                            }
                            Err(e) => {
                                let _ = tx
                                    .send(crate::plugin::manager::PluginRequest::Notify {
                                        title: t("plugin_toast_update_err_title"),
                                        msg: t("plugin_toast_update_err")
                                            .replace("{}", &name_clone)
                                            .replace("{:?}", &format!("{:?}", e)),
                                        level: "error".to_string(),
                                    })
                                    .await;
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
                            let _ = tx
                                .send(crate::plugin::manager::PluginRequest::Notify {
                                    title: t("plugin_toast_update_all_title"),
                                    msg: t("plugin_toast_update_all_ok"),
                                    level: "info".to_string(),
                                })
                                .await;
                        }
                        Err(e) => {
                            let _ = tx
                                .send(crate::plugin::manager::PluginRequest::Notify {
                                    title: t("plugin_toast_update_all_err_title"),
                                    msg: t("plugin_toast_update_all_err")
                                        .replace("{:?}", &format!("{:?}", e)),
                                    level: "error".to_string(),
                                })
                                .await;
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
                        tokio::runtime::Handle::current()
                            .block_on(crate::plugin::updater::fetch_index())
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
                                        let _ = tx
                                            .send(crate::plugin::manager::PluginRequest::Notify {
                                                title: t("plugin_toast_install_title"),
                                                msg: t("plugin_toast_install_ok")
                                                    .replace("{}", &name_clone),
                                                level: "info".to_string(),
                                            })
                                            .await;
                                    }
                                    Err(e) => {
                                        let _ = tx
                                            .send(crate::plugin::manager::PluginRequest::Notify {
                                                title: t("plugin_toast_install_err_title"),
                                                msg: t("plugin_toast_install_err")
                                                    .replace("{}", &name_clone)
                                                    .replace("{:?}", &format!("{:?}", e)),
                                                level: "error".to_string(),
                                            })
                                            .await;
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
                    if dev_wizard_step == 1 {
                        let name = search_query.clone().trim().to_string();
                        if !name.is_empty() {
                            dev_wizard_data.push(name);
                            search_query.clear();
                            dev_wizard_step = 2; // Prompt for description
                        }
                    } else if dev_wizard_step == 2 {
                        let desc = search_query.clone().trim().to_string();
                        dev_wizard_data.push(desc);
                        search_query.clear();
                        dev_wizard_step = 3; // Prompt for author
                    } else if dev_wizard_step == 3 {
                        let author = search_query.clone().trim().to_string();
                        dev_wizard_data.push(author);
                        search_query.clear();
                        editing_query = false;
                        dev_wizard_step = 0;

                        let plugins_dev_dir = &context.config.settings.plugins_dev_dir;
                        let folder_name = if dev_wizard_data[0].ends_with(".pairee") {
                            dev_wizard_data[0].clone()
                        } else {
                            format!("{}.pairee", dev_wizard_data[0])
                        };
                        let target_path =
                            std::path::PathBuf::from(plugins_dev_dir).join(&folder_name);
                        let _ = std::fs::create_dir_all(&target_path);
                        if let Ok(current_dir) = std::env::current_dir() {
                            if std::env::set_current_dir(plugins_dev_dir).is_ok() {
                                match crate::plugin::developer_tool::init(&folder_name, &dev_wizard_data[1], &dev_wizard_data[2], false) {
                                    Ok(_) => {
                                        let name_without_suffix = folder_name
                                            .strip_suffix(".pairee")
                                            .unwrap_or(&folder_name);
                                        dev_results = t("plugin_dev_init_ok")
                                            .replace("{}", name_without_suffix)
                                            .replace("{:?}", &format!("{:?}", target_path));
                                    }
                                    Err(e) => {
                                        dev_results = t("plugin_dev_init_err")
                                            .replace("{:?}", &format!("{:?}", e));
                                    }
                                }
                                let _ = std::env::set_current_dir(current_dir);
                            }
                        }
                        dev_wizard_data.clear();
                        installed = reload_installed_plugins(context, &None);
                    } else if dev_wizard_step == 5 {
                        let commit_msg = search_query.clone().trim().to_string();
                        if !commit_msg.is_empty() {
                            dev_wizard_data.push(commit_msg);
                            search_query.clear();
                            dev_wizard_step = 6; // Prompt for GitHub Token
                        }
                    } else if dev_wizard_step == 6 {
                        let token = search_query.clone().trim().to_string();
                        let plugin_path_str = dev_wizard_data[0].clone();
                        let commit_msg = dev_wizard_data[1].clone();
                        dev_wizard_data.clear();
                        editing_query = false;
                        dev_wizard_step = 0;
                        search_query.clear();

                        // 1. Commit locally first
                        let plugin_path = std::path::PathBuf::from(&plugin_path_str);
                        let manifest_path = plugin_path.join("manifest.toml");
                        let mut plugin_name = String::new();
                        if let Ok(manifest_content) = std::fs::read_to_string(&manifest_path) {
                            if let Ok(manifest) = crate::plugin::loader::PluginManifest::parse(&manifest_content) {
                                plugin_name = manifest.name;
                            }
                        }

                        let mut local_err = None;
                        match crate::plugin::developer_tool::package_to_registry(&plugin_path) {
                            Ok(_) => {
                                if let Err(e) = crate::plugin::developer_tool::commit_registry_changes(&commit_msg) {
                                    local_err = Some(format!("Failed to commit changes locally: {:?}", e));
                                }
                            }
                            Err(e) => {
                                local_err = Some(format!("Failed to package plugin to registry: {:?}", e));
                            }
                        }

                        if let Some(err) = local_err {
                            dev_results = err;
                        } else {
                            let temp_dir = crate::config::paths::get_cache_dir().join("temp_registry");
                            if token.is_empty() {
                                dev_results = format!(
                                    "Staged and committed locally!\n\nNo GitHub token provided. To submit manually:\n\n\
                                     1. Fork the FittyAr/Pairee repository on GitHub.\n\
                                     2. Run the following commands in your terminal:\n\n\
                                        cd \"{}\"\n\
                                        git remote add myfork <URL_TO_YOUR_FORK>\n\
                                        git push myfork plugin-registry\n\n\
                                     3. Create a Pull Request from your fork's plugin-registry branch to FittyAr/Pairee:plugin-registry.",
                                    temp_dir.display()
                                );
                            } else {
                                let tx = crate::plugin::PluginManager::get_sender();
                                tokio::spawn(async move {
                                    match crate::plugin::developer_tool::run_automatic_submit(&token, &commit_msg, &plugin_name).await {
                                        Ok(msg) => {
                                            let _ = tx.send(crate::plugin::manager::PluginRequest::Notify {
                                                title: "Plugin Submitted".to_string(),
                                                msg,
                                                level: "info".to_string(),
                                            }).await;
                                        }
                                        Err(e) => {
                                            let _ = tx.send(crate::plugin::manager::PluginRequest::Notify {
                                                title: "Submission Failed".to_string(),
                                                msg: format!("{:?}", e),
                                                level: "error".to_string(),
                                            }).await;
                                        }
                                    }
                                });
                                dev_results = "Staged and committed locally! Automating remote fork & push in background... Check status in notifications.".to_string();
                            }
                        }
                        installed = reload_installed_plugins(context, &None);
                    }
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    if cursor_idx == 0 {
                        cursor_idx = 4;
                    } else {
                        cursor_idx -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    if cursor_idx >= 4 {
                        cursor_idx = 0;
                    } else {
                        cursor_idx += 1;
                    }
                }
                KeyCode::Enter => {
                    let plugins_dev_dir = &context.config.settings.plugins_dev_dir;
                    match cursor_idx {
                        0 => {
                            // Init Plugin
                            editing_query = true;
                            search_query = String::new();
                            dev_results = String::new();
                            dev_wizard_step = 1;
                            dev_wizard_data = Vec::new();
                        }
                        1 => {
                            // Lint all development plugins in plugins_dev_dir
                            let mut report = String::new();
                            let mut found_any = false;
                            if let Ok(entries) = std::fs::read_dir(plugins_dev_dir) {
                                for entry in entries.filter_map(Result::ok) {
                                    let path = entry.path();
                                    if path.is_dir() && path.join("manifest.toml").exists() {
                                        let folder_name = path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("")
                                            .to_string();
                                        if !folder_name.ends_with(".pairee") {
                                            continue;
                                        }
                                        found_any = true;
                                        let name = folder_name
                                            .strip_suffix(".pairee")
                                            .unwrap_or(&folder_name)
                                            .to_string();
                                        let manifest_path = path.join("manifest.toml");
                                        let main_path = path.join("main.lua");

                                        report.push_str(
                                            &t("plugin_dev_lint_start").replace("{}", &name),
                                        );
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
                                            if let Ok(lua_code) =
                                                std::fs::read_to_string(&main_path)
                                            {
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
                                        report.push_str(
                                            "\n────────────────────────────────────────\n\n",
                                        );
                                    }
                                }
                            }
                            if !found_any {
                                report = format!(
                                    "No development plugins found in developer directory:\n{:?}",
                                    plugins_dev_dir
                                );
                            }
                            dev_results = report;
                        }
                        2 => {
                            // Package all development plugins in plugins_dev_dir
                            let mut report = String::new();
                            let mut found_any = false;
                            if let Ok(entries) = std::fs::read_dir(plugins_dev_dir) {
                                for entry in entries.filter_map(Result::ok) {
                                    let path = entry.path();
                                    if path.is_dir() && path.join("manifest.toml").exists() {
                                        let folder_name = path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("")
                                            .to_string();
                                        if !folder_name.ends_with(".pairee") {
                                            continue;
                                        }
                                        found_any = true;
                                        report.push_str(&format!("Packaging '{}'...\n", folder_name));
                                        match crate::plugin::developer_tool::package_to_registry(&path) {
                                            Ok(msg) => {
                                                report.push_str(&format!("✓ {}\n", msg));
                                            }
                                            Err(e) => {
                                                report.push_str(&format!("✗ Failed: {:?}\n", e));
                                            }
                                        }
                                        report.push_str(
                                            "\n────────────────────────────────────────\n\n",
                                        );
                                    }
                                }
                            }
                            if !found_any {
                                report = format!(
                                    "No development plugins found in developer directory:\n{:?}",
                                    plugins_dev_dir
                                );
                            }
                            dev_results = report;
                        }
                        3 => {
                            // Install all development plugins to local plugins folder
                            let mut report = String::new();
                            let mut found_any = false;
                            let dest_base = crate::config::paths::get_config_dir().join("plugins");
                            let mut lock = crate::plugin::updater::read_lockfile();
                            if let Ok(entries) = std::fs::read_dir(plugins_dev_dir) {
                                for entry in entries.filter_map(Result::ok) {
                                    let path = entry.path();
                                    if path.is_dir() && path.join("manifest.toml").exists() {
                                        let folder_name = path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("")
                                            .to_string();
                                        if !folder_name.ends_with(".pairee") {
                                            continue;
                                        }
                                        let manifest_path = path.join("manifest.toml");
                                        if let Ok(manifest_content) =
                                            std::fs::read_to_string(&manifest_path)
                                        {
                                            if let Ok(manifest) =
                                                crate::plugin::loader::PluginManifest::parse(
                                                    &manifest_content,
                                                )
                                            {
                                                found_any = true;
                                                let name = manifest.name.clone();
                                                let version = manifest.version.clone();
                                                let dest_dir =
                                                    dest_base.join(format!("{}.pairee", name));
                                                let _ = std::fs::create_dir_all(&dest_dir);

                                                let mut copied_files = Vec::new();
                                                for (rel_path_str, src_file_path) in
                                                    crate::plugin::loader::get_plugin_files(&path)
                                                {
                                                    let dest_file_path =
                                                        dest_dir.join(&rel_path_str);
                                                    if let Some(parent) = dest_file_path.parent() {
                                                        let _ = std::fs::create_dir_all(parent);
                                                    }
                                                    if std::fs::copy(&src_file_path, &dest_file_path)
                                                        .is_ok()
                                                    {
                                                        copied_files.push(rel_path_str);
                                                    }
                                                }

                                                let mut files_hash =
                                                    std::collections::HashMap::new();
                                                for (rel, p) in
                                                    crate::plugin::loader::get_plugin_files(
                                                        &dest_dir,
                                                    )
                                                {
                                                    if let Ok(h) =
                                                        crate::update::downloader::compute_sha256(
                                                            &p,
                                                        )
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

                                                report.push_str(&format!("✓ Installed '{}' locally (copied {} file(s))\n", name, copied_files.len()));
                                            }
                                        }
                                    }
                                }
                            }
                            if !found_any {
                                report = format!(
                                    "No development plugins found to install in developer directory:\n{:?}",
                                    plugins_dev_dir
                                );
                            } else {
                                let _ = crate::plugin::updater::write_lockfile(&lock);
                                report.push_str("\nLocal plugins synced successfully! Restart Pairee or reload plugins to apply.");
                            }
                            dev_results = report;
                            installed = reload_installed_plugins(context, &None);
                        }
                        4 => {
                            // Submit Plugin
                            let mut found_path = None;
                            if let Ok(entries) = std::fs::read_dir(plugins_dev_dir) {
                                for entry in entries.filter_map(Result::ok) {
                                    let path = entry.path();
                                    if path.is_dir() && path.join("manifest.toml").exists() {
                                        let folder_name = path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("")
                                            .to_string();
                                        if folder_name.ends_with(".pairee") {
                                            found_path = Some(path);
                                            break;
                                        }
                                    }
                                }
                            }

                            if let Some(plugin_path) = found_path {
                                match crate::plugin::developer_tool::validate_for_publish(&plugin_path) {
                                    Ok(_) => {
                                        editing_query = true;
                                        search_query = String::new();
                                        dev_results = String::new();
                                        dev_wizard_step = 5;
                                        dev_wizard_data = vec![plugin_path.to_string_lossy().to_string()];
                                    }
                                    Err(err_msg) => {
                                        dev_results = err_msg;
                                    }
                                }
                            } else {
                                dev_results = format!(
                                    "No development plugins found to submit in developer directory:\n{:?}",
                                    plugins_dev_dir
                                );
                            }
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
        dev_wizard_step,
        dev_wizard_data,
    });

    Ok(None)
}
