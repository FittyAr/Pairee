use super::reload_installed_plugins;
use crate::app::context::AppContext;
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_dev(
    key: KeyEvent,
    context: &mut AppContext,
    cursor_idx: &mut usize,
    installed: &mut Vec<(String, String, bool, bool, Option<String>)>,
    search_query: &mut String,
    editing_query: &mut bool,
    dev_results: &mut String,
    dev_wizard_step: &mut usize,
    dev_wizard_data: &mut Vec<String>,
) {
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
                                        .replace("{:?}", &format!("{:?}", target_path));
                                }
                                Err(e) => {
                                    *dev_results = t("plugin_dev_init_err")
                                        .replace("{:?}", &format!("{:?}", e));
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
                if *cursor_idx == 0 {
                    *cursor_idx = 4;
                } else {
                    *cursor_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if *cursor_idx >= 4 {
                    *cursor_idx = 0;
                } else {
                    *cursor_idx += 1;
                }
            }
            KeyCode::Enter => {
                let plugins_dev_dir = &context.config.settings.plugins_dev_dir;
                match *cursor_idx {
                    0 => {
                        // Init Plugin
                        *editing_query = true;
                        *search_query = String::new();
                        *dev_results = String::new();
                        *dev_wizard_step = 1;
                        *dev_wizard_data = Vec::new();
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

                                    report
                                        .push_str(&t("plugin_dev_lint_start").replace("{}", &name));
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
                                    report
                                        .push_str("\n────────────────────────────────────────\n\n");
                                }
                            }
                        }
                        if !found_any {
                            report = t("plugin_dev_no_plugins_found")
                                .replace("{:?}", &format!("{:?}", plugins_dev_dir));
                        }
                        *dev_results = report;
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
                                    report.push_str(&format!(
                                        "{} '{}'...\n",
                                        t("plugin_dev_pack_start").trim(),
                                        folder_name
                                    ));
                                    match crate::plugin::developer_tool::package_to_registry(&path)
                                    {
                                        Ok(msg) => {
                                            report.push_str(&format!("✓ {}\n", msg));
                                        }
                                        Err(e) => {
                                            report.push_str(&format!("✗ Failed: {:?}\n", e));
                                        }
                                    }
                                    report
                                        .push_str("\n────────────────────────────────────────\n\n");
                                }
                            }
                        }
                        if !found_any {
                            report = t("plugin_dev_no_plugins_found")
                                .replace("{:?}", &format!("{:?}", plugins_dev_dir));
                        }
                        *dev_results = report;
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
                                        }
                                    }
                                }
                            }
                        }
                        if !found_any {
                            report = t("plugin_dev_no_plugins_to_install")
                                .replace("{:?}", &format!("{:?}", plugins_dev_dir));
                        } else {
                            let _ = crate::plugin::updater::write_lockfile(&lock);
                            report.push_str(&format!("\n{}", t("plugin_dev_local_sync_ok")));
                        }
                        *dev_results = report;
                        *installed = reload_installed_plugins(context, &None);
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
                            match crate::plugin::developer_tool::validate_for_publish(&plugin_path)
                            {
                                Ok(_) => {
                                    *editing_query = true;
                                    *search_query = String::new();
                                    *dev_results = String::new();
                                    *dev_wizard_step = 5;
                                    *dev_wizard_data =
                                        vec![plugin_path.to_string_lossy().to_string()];
                                }
                                Err(err_msg) => {
                                    *dev_results = err_msg;
                                }
                            }
                        } else {
                            *dev_results = t("plugin_dev_no_plugins_found")
                                .replace("{:?}", &format!("{:?}", plugins_dev_dir));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
