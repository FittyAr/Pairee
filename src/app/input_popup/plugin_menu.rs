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
                    editing_query = false;
                    let is_submit = dev_results == "GITHUB_SUBMIT_TOKEN_PROMPT";
                    if is_submit {
                        let token = search_query.clone();
                        let tx = crate::plugin::PluginManager::get_sender();
                        tokio::spawn(async move {
                            let client = reqwest::Client::builder().build();
                            if let Ok(c) = client {
                                let fork_url = "https://api.github.com/repos/FittyAr/Pairee/forks";
                                let resp = c
                                    .post(fork_url)
                                    .header("User-Agent", "Pairee-Submit-Wizard")
                                    .header("Authorization", format!("token {}", token))
                                    .send()
                                    .await;
                                match resp {
                                    Ok(r)
                                        if r.status().is_success()
                                            || r.status() == reqwest::StatusCode::ACCEPTED =>
                                    {
                                        let _ = tx.send(crate::plugin::manager::PluginRequest::Notify {
                                            title: "Plugin Submitted".to_string(),
                                            msg: "Fork created successfully! Submit PR from your branch.".to_string(),
                                            level: "info".to_string(),
                                        }).await;
                                    }
                                    Ok(r) => {
                                        let _ = tx
                                            .send(crate::plugin::manager::PluginRequest::Notify {
                                                title: "Submission Failed".to_string(),
                                                msg: format!("HTTP Error: {}", r.status()),
                                                level: "error".to_string(),
                                            })
                                            .await;
                                    }
                                    Err(e) => {
                                        let _ = tx
                                            .send(crate::plugin::manager::PluginRequest::Notify {
                                                title: "Submission Failed".to_string(),
                                                msg: format!("{:?}", e),
                                                level: "error".to_string(),
                                            })
                                            .await;
                                    }
                                }
                            }
                        });
                        dev_results = "Submission request initiated in background... Check status in notifications.".to_string();
                    } else {
                        let plugins_dev_dir = &context.config.settings.plugins_dev_dir;
                        let folder_name = if search_query.ends_with(".pairee") {
                            search_query.clone()
                        } else {
                            format!("{}.pairee", search_query)
                        };
                        let target_path =
                            std::path::PathBuf::from(plugins_dev_dir).join(&folder_name);
                        let _ = std::fs::create_dir_all(&target_path);
                        if let Ok(current_dir) = std::env::current_dir() {
                            if std::env::set_current_dir(plugins_dev_dir).is_ok() {
                                match crate::plugin::developer_tool::init(&folder_name, false) {
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
                    }
                    installed = reload_installed_plugins(context, &None);
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
                                        let name = folder_name
                                            .strip_suffix(".pairee")
                                            .unwrap_or(&folder_name)
                                            .to_string();
                                        report.push_str(
                                            &t("plugin_dev_pack_start").replace("{}", &name),
                                        );
                                        let mut files_hash = std::collections::HashMap::new();
                                        for (rel, p) in
                                            crate::plugin::loader::get_plugin_files(&path)
                                        {
                                            if let Ok(hash) =
                                                crate::update::downloader::compute_sha256(&p)
                                            {
                                                files_hash.insert(rel, hash);
                                            }
                                        }
                                        report.push_str(&t("plugin_dev_pack_gen"));
                                        report.push_str(&format!("[plugins.{}]\n", name));
                                        report.push_str(&format!("name = \"{}\"\n", name));
                                        report.push_str("version = \"0.1.0\"\n");
                                        report.push_str("files = {\n");
                                        for (f, h) in files_hash {
                                            report
                                                .push_str(&format!("    \"{}\" = \"{}\",\n", f, h));
                                        }
                                        report.push_str("}\n");
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
                                                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                                                    for sub_entry in
                                                        sub_entries.filter_map(Result::ok)
                                                    {
                                                        let sub_path = sub_entry.path();
                                                        if sub_path.is_file() {
                                                            if let Some(filename) =
                                                                sub_path.file_name()
                                                            {
                                                                let _ = std::fs::copy(
                                                                    &sub_path,
                                                                    dest_dir.join(filename),
                                                                );
                                                                copied_files.push(
                                                                    filename
                                                                        .to_string_lossy()
                                                                        .into_owned(),
                                                                );
                                                            }
                                                        } else if sub_path.is_dir()
                                                            && sub_path
                                                                .file_name()
                                                                .map(|n| n == "lang")
                                                                .unwrap_or(false)
                                                        {
                                                            let lang_dest = dest_dir.join("lang");
                                                            let _ =
                                                                std::fs::create_dir_all(&lang_dest);
                                                            if let Ok(lang_entries) =
                                                                std::fs::read_dir(&sub_path)
                                                            {
                                                                for le in lang_entries
                                                                    .filter_map(Result::ok)
                                                                {
                                                                    if le.path().is_file() {
                                                                        if let Some(fn_lang) =
                                                                            le.path().file_name()
                                                                        {
                                                                            let _ = std::fs::copy(
                                                                                le.path(),
                                                                                lang_dest
                                                                                    .join(fn_lang),
                                                                            );
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
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
                            editing_query = true;
                            search_query = String::new();
                            dev_results = "GITHUB_SUBMIT_TOKEN_PROMPT".to_string();
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
