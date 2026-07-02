use super::reload_installed_plugins;
use crate::app::context::AppContext;
use crate::app::state::{AppState, DevProgress, PopupType};
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};
use std::path::{Path, PathBuf};

/// Number of options in the Developer Tools menu (0-8).
/// Indices 6-8 are the "move panel to folder" shortcuts.
const DEV_OPT_COUNT: usize = 9;

/// Returns true if the given index is the "Init" slot (currently index 1),
/// which is only available when no active plugin is selected.
fn is_init_option(idx: usize) -> bool {
    idx == 1
}

/// Move the currently active panel to `path` and close the popup so the
/// user lands directly on the file list.
fn move_active_panel_to(state: &mut AppState, path: PathBuf, show_hidden: bool) {
    state.get_active_panel_mut().current_path = path;
    state.refresh_both_panels(show_hidden);
    state.active_popup = None;
}

/// Start a Developer Tools async operation: wire up the progress channel on
/// `state`, flip the popup into the "loading" state, and return the sender
/// half so the caller can spawn the work.
fn begin_dev_op(
    state: &mut AppState,
    initial_status: String,
) -> tokio::sync::mpsc::UnboundedSender<DevProgress> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<DevProgress>();
    state.dev_progress_rx = Some(rx);
    if let Some(PopupType::PluginMenu {
        dev_loading,
        dev_loading_status,
        dev_loading_progress,
        ..
    }) = &mut state.active_popup
    {
        *dev_loading = true;
        *dev_loading_status = initial_status;
        *dev_loading_progress = None;
    }
    tx
}

/// Emit a coarse status update over the given progress sender (if any).
fn progress_status(tx: &Option<tokio::sync::mpsc::UnboundedSender<DevProgress>>, status: String) {
    if let Some(tx) = tx {
        let _ = tx.send(DevProgress {
            status,
            current: None,
            total: None,
            done: false,
            result: None,
            error: None,
        });
    }
}

/// Compute the absolute path of the active development plugin, falling
/// back to `plugins_dev_dir` if there is no active plugin or the folder
/// no longer exists.
fn dev_plugin_dir(context: &AppContext) -> PathBuf {
    let base = PathBuf::from(&context.config.settings.plugins_dev_dir);
    match &context.config.settings.active_dev_plugin {
        Some(name) if !name.is_empty() => {
            let candidate = if Path::new(name).is_absolute() {
                PathBuf::from(name)
            } else {
                base.join(name)
            };
            if candidate.exists() { candidate } else { base }
        }
        _ => base,
    }
}

/// Compute the absolute path of the packaged plugin inside the temp
/// registry clone (`cache_dir/temp_registry/registry/plugins/...`).
/// Returns `None` if the manifest cannot be parsed.
fn packaged_plugin_dir(active_plugin: &str) -> Option<PathBuf> {
    let plugin_path = PathBuf::from(active_plugin);
    let manifest_path = plugin_path.join("manifest.toml");
    let content = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest = crate::plugin::loader::PluginManifest::parse(&content).ok()?;
    let name = manifest.name;
    let author = manifest.author.as_deref().unwrap_or("unknown").trim();
    let author = if author.is_empty() { "unknown" } else { author };
    let first_char = author.chars().next().unwrap_or('u').to_ascii_lowercase();
    let first_char_str = if first_char.is_ascii_alphabetic() {
        first_char.to_string()
    } else {
        "_".to_string()
    };
    Some(
        crate::config::paths::get_cache_dir()
            .join("temp_registry")
            .join("registry")
            .join("plugins")
            .join(first_char_str)
            .join(author)
            .join(&name),
    )
}

/// Returns true if a Developer Tools operation is currently in flight.
fn dev_op_running(state: &AppState) -> bool {
    if let Some(PopupType::PluginMenu { dev_loading, .. }) = &state.active_popup {
        *dev_loading
    } else {
        false
    }
}

pub fn handle_dev(
    key: KeyEvent,
    state: &mut AppState,
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

                    // === Async init wizard step ===
                    let name = dev_wizard_data[0].clone();
                    let desc = dev_wizard_data[1].clone();
                    let author = dev_wizard_data[2].clone();
                    dev_wizard_data.clear();
                    let plugins_dev_dir = context.config.settings.plugins_dev_dir.clone();
                    let folder_name = if name.ends_with(".pairee") {
                        name.clone()
                    } else {
                        format!("{}.pairee", name)
                    };
                    let target_path = PathBuf::from(&plugins_dev_dir).join(&folder_name);

                    let _ = std::fs::create_dir_all(&target_path);
                    *dev_results = format!(
                        "{} '{}'…",
                        t("plugin_dev_progress_initializing"),
                        folder_name
                    );

                    let tx = begin_dev_op(state, t("plugin_dev_progress_creating_dir"));
                    tokio::task::spawn_blocking(move || {
                        let prev = std::env::current_dir().ok();
                        let _ = std::env::set_current_dir(&plugins_dev_dir);
                        let res = crate::plugin::developer_tool::init_with_progress(
                            &folder_name,
                            &desc,
                            &author,
                            false,
                            Some(tx.clone()),
                        );
                        if let Some(prev) = prev {
                            let _ = std::env::set_current_dir(&prev);
                        }
                        match res {
                            Ok(_) => {
                                let name_without_suffix = folder_name
                                    .strip_suffix(".pairee")
                                    .unwrap_or(&folder_name)
                                    .to_string();
                                let result_text = t("plugin_dev_init_ok")
                                    .replace("{}", &name_without_suffix)
                                    .replace("{:?}", &target_path.to_string_lossy());
                                let result_text = format!(
                                    "{}\n\n{}",
                                    result_text,
                                    t("plugin_dev_init_select_hint")
                                );
                                crate::plugin::developer_tool::progress_finish(
                                    Some(tx),
                                    Some(result_text),
                                    None,
                                );
                            }
                            Err(e) => {
                                let err =
                                    t("plugin_dev_init_err").replace("{:?}", &format!("{}", e));
                                crate::plugin::developer_tool::progress_finish(
                                    Some(tx),
                                    None,
                                    Some(err),
                                );
                            }
                        }
                    });
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

                    *dev_results = format!(
                        "{} '{}'…",
                        t("plugin_dev_progress_submitting"),
                        plugin_path_str
                    );

                    let tx = begin_dev_op(state, t("plugin_dev_progress_packaging"));
                    let plugin_path = PathBuf::from(&plugin_path_str);
                    let commit_msg_for_blocking = commit_msg.clone();
                    let plugin_path_for_blocking = plugin_path.clone();

                    // Phase 1: package + commit (synchronous, on the blocking pool)
                    tokio::task::spawn_blocking(move || {
                        let mut local_err: Option<String> = None;
                        match crate::plugin::developer_tool::package_to_registry_with_progress(
                            &plugin_path_for_blocking,
                            Some(tx.clone()),
                        ) {
                            Ok(_) => {
                                if let Err(e) =
                                    crate::plugin::developer_tool::commit_registry_changes_with_progress(
                                        &commit_msg_for_blocking,
                                        Some(tx.clone()),
                                    )
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
                            crate::plugin::developer_tool::progress_finish(
                                Some(tx),
                                None,
                                Some(err),
                            );
                            return;
                        }

                        if token.is_empty() {
                            let temp_dir =
                                crate::config::paths::get_cache_dir().join("temp_registry");
                            let result = t("plugin_dev_no_token_inst")
                                .replace("{}", &temp_dir.display().to_string());
                            crate::plugin::developer_tool::progress_finish(
                                Some(tx),
                                Some(result),
                                None,
                            );
                            return;
                        }

                        // Phase 2: GitHub fork + push + PR (true async)
                        let tx_for_async = tx.clone();
                        let commit_msg_async = commit_msg.clone();
                        let manifest_path = plugin_path.join("manifest.toml");
                        let mut plugin_name = String::new();
                        if let Ok(manifest_content) = std::fs::read_to_string(&manifest_path) {
                            if let Ok(manifest) =
                                crate::plugin::loader::PluginManifest::parse(&manifest_content)
                            {
                                plugin_name = manifest.name;
                            }
                        }

                        tokio::spawn(async move {
                            let notify_tx = crate::plugin::PluginManager::get_sender();
                            match crate::plugin::developer_tool::run_automatic_submit(
                                &token,
                                &commit_msg_async,
                                &plugin_name,
                            )
                            .await
                            {
                                Ok(msg) => {
                                    let _ = notify_tx
                                        .send(crate::plugin::manager::PluginRequest::Notify {
                                            title: t("plugin_dev_toast_submitted_title"),
                                            msg,
                                            level: "info".to_string(),
                                        })
                                        .await;
                                    crate::plugin::developer_tool::progress_finish(
                                        Some(tx_for_async),
                                        Some(t("plugin_dev_fork_push_bg").to_string()),
                                        None,
                                    );
                                }
                                Err(e) => {
                                    let _ = notify_tx
                                        .send(crate::plugin::manager::PluginRequest::Notify {
                                            title: t("plugin_dev_toast_submit_fail_title"),
                                            msg: format!("{:?}", e),
                                            level: "error".to_string(),
                                        })
                                        .await;
                                    crate::plugin::developer_tool::progress_finish(
                                        Some(tx_for_async),
                                        None,
                                        Some(format!("{:?}", e)),
                                    );
                                }
                            }
                        });
                    });
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
                    *cursor_idx = DEV_OPT_COUNT - 1;
                } else if has_active && *cursor_idx == 2 {
                    *cursor_idx = 0; // Skip 1 (Init) because it's disabled
                } else {
                    *cursor_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                let has_active = context.config.settings.active_dev_plugin.is_some();
                if *cursor_idx >= DEV_OPT_COUNT - 1 {
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
                let plugins_dev_dir = context.config.settings.plugins_dev_dir.clone();
                let active_plugin = context.config.settings.active_dev_plugin.clone();

                // Refuse to start a new long-running op while one is in
                // progress.
                if dev_op_running(state) {
                    *dev_results = t("plugin_dev_op_in_progress");
                    return;
                }

                match *cursor_idx {
                    0 => {
                        // === Option 0: Select / Change / Deselect active plugin ===
                        // The scan runs on the blocking pool to keep the UI
                        // responsive even when the dev folder is large.
                        *dev_results = t("plugin_dev_progress_scanning_plugins");
                        let left = left_panel_path.to_path_buf();
                        let right = right_panel_path.to_path_buf();
                        let tx = crate::plugin::PluginManager::get_sender();
                        tokio::task::spawn_blocking(move || {
                            let mut options = Vec::new();
                            options.push(("[Deselect / None]".to_string(), "deselect".to_string()));

                            if let Ok(entries) = std::fs::read_dir(&plugins_dev_dir) {
                                for entry in entries.filter_map(Result::ok) {
                                    let path = entry.path();
                                    if path.is_dir() && path.join("manifest.toml").exists() {
                                        if let Some(name) =
                                            path.file_name().and_then(|n| n.to_str())
                                        {
                                            options.push((name.to_string(), name.to_string()));
                                        }
                                    }
                                }
                            }

                            if left.join("manifest.toml").exists() {
                                if let Some(name) = left.file_name().and_then(|n| n.to_str()) {
                                    options.push((
                                        format!("[Panel 1] {} ({})", name, left.display()),
                                        left.to_string_lossy().to_string(),
                                    ));
                                }
                            }

                            if right.join("manifest.toml").exists() {
                                if let Some(name) = right.file_name().and_then(|n| n.to_str()) {
                                    options.push((
                                        format!("[Panel 2] {} ({})", name, right.display()),
                                        right.to_string_lossy().to_string(),
                                    ));
                                }
                            }

                            let _ = tx.blocking_send(
                                crate::plugin::manager::PluginRequest::DevPluginScan { options },
                            );
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
                        // Lint active plugin (async)
                        if let Some(plugin_folder) = active_plugin.clone() {
                            let path = if Path::new(&plugin_folder).is_absolute() {
                                PathBuf::from(&plugin_folder)
                            } else {
                                PathBuf::from(&plugins_dev_dir).join(&plugin_folder)
                            };
                            if !path.exists()
                                || !path.is_dir()
                                || !path.join("manifest.toml").exists()
                            {
                                *dev_results = format!(
                                    "Error: Plugin directory '{}' no longer exists.",
                                    plugin_folder
                                );
                            } else {
                                let name = plugin_folder
                                    .strip_suffix(".pairee")
                                    .unwrap_or(&plugin_folder)
                                    .to_string();
                                *dev_results = t("plugin_dev_lint_start").replace("{}", &name);
                                let tx = begin_dev_op(state, t("plugin_dev_progress_linting"));
                                let path_for_task = path.clone();
                                let name_for_result = name.clone();
                                tokio::task::spawn_blocking(move || {
                                    let prev = std::env::current_dir().ok();
                                    let _ = std::env::set_current_dir(&path_for_task);
                                    let res = crate::plugin::developer_tool::lint_with_progress(
                                        Some(tx.clone()),
                                    );
                                    if let Some(prev) = prev {
                                        let _ = std::env::set_current_dir(&prev);
                                    }
                                    match res {
                                        Ok(_) => {
                                            let result = format!(
                                                "{} '{}' {}",
                                                t("plugin_dev_lint_complete_for"),
                                                name_for_result,
                                                t("plugin_dev_lint_complete_tail")
                                            );
                                            crate::plugin::developer_tool::progress_finish(
                                                Some(tx),
                                                Some(result),
                                                None,
                                            );
                                        }
                                        Err(e) => {
                                            crate::plugin::developer_tool::progress_finish(
                                                Some(tx),
                                                None,
                                                Some(format!("{:?}", e)),
                                            );
                                        }
                                    }
                                });
                            }
                        } else {
                            *dev_results = t("plugin_dev_no_active_err");
                        }
                    }
                    3 => {
                        // Package active plugin (async)
                        if let Some(plugin_folder) = active_plugin.clone() {
                            let path = if Path::new(&plugin_folder).is_absolute() {
                                PathBuf::from(&plugin_folder)
                            } else {
                                PathBuf::from(&plugins_dev_dir).join(&plugin_folder)
                            };
                            if !path.exists()
                                || !path.is_dir()
                                || !path.join("manifest.toml").exists()
                            {
                                *dev_results = format!(
                                    "Error: Plugin directory '{}' no longer exists.",
                                    plugin_folder
                                );
                            } else {
                                *dev_results = format!(
                                    "{} '{}'…",
                                    t("plugin_dev_pack_start").trim(),
                                    plugin_folder
                                );
                                let tx =
                                    begin_dev_op(state, t("plugin_dev_progress_fetching_registry"));
                                let path_for_task = path.clone();
                                let name_for_result = plugin_folder.clone();
                                tokio::task::spawn_blocking(move || {
                                    match crate::plugin::developer_tool::package_to_registry_with_progress(
                                        &path_for_task,
                                        Some(tx.clone()),
                                    ) {
                                        Ok(msg) => {
                                            let result = format!(
                                                "✓ {}\n\n{}",
                                                msg,
                                                t("plugin_dev_pack_done_tail")
                                                    .replace("{}", &name_for_result)
                                            );
                                            crate::plugin::developer_tool::progress_finish(
                                                Some(tx),
                                                Some(result),
                                                None,
                                            );
                                        }
                                        Err(e) => {
                                            crate::plugin::developer_tool::progress_finish(
                                                Some(tx),
                                                None,
                                                Some(format!("✗ Failed: {:?}", e)),
                                            );
                                        }
                                    }
                                });
                            }
                        } else {
                            *dev_results = t("plugin_dev_no_active_err");
                        }
                    }
                    4 => {
                        // Install active plugin locally (async with per-file progress)
                        if let Some(plugin_folder) = active_plugin.clone() {
                            let path = if Path::new(&plugin_folder).is_absolute() {
                                PathBuf::from(&plugin_folder)
                            } else {
                                PathBuf::from(&plugins_dev_dir).join(&plugin_folder)
                            };
                            if !path.exists()
                                || !path.is_dir()
                                || !path.join("manifest.toml").exists()
                            {
                                *dev_results = format!(
                                    "Error: Plugin directory '{}' no longer exists.",
                                    plugin_folder
                                );
                            } else {
                                *dev_results = format!(
                                    "{} '{}'…",
                                    t("plugin_dev_install_start"),
                                    plugin_folder
                                );
                                let tx =
                                    begin_dev_op(state, t("plugin_dev_progress_copying_files"));
                                let path_for_task = path.clone();
                                tokio::task::spawn_blocking(move || {
                                    use crate::plugin::developer_tool::progress_progress;
                                    let manifest_path = path_for_task.join("manifest.toml");
                                    let res: Result<String, String> = (|| {
                                        let content = std::fs::read_to_string(&manifest_path)
                                            .map_err(|e| format!("{:?}", e))?;
                                        let manifest =
                                            crate::plugin::loader::PluginManifest::parse(&content)
                                                .map_err(|e| format!("{:?}", e))?;
                                        let name = manifest.name.clone();
                                        let version = manifest.version.clone();
                                        let dest_base =
                                            crate::config::paths::get_config_dir().join("plugins");
                                        let mut lock = crate::plugin::updater::read_lockfile();
                                        let dest_dir = dest_base.join(format!("{}.pairee", name));
                                        let _ = std::fs::create_dir_all(&dest_dir);

                                        let files =
                                            crate::plugin::loader::get_plugin_files(&path_for_task);
                                        let total = files.len().max(1);
                                        let mut copied_files = Vec::new();
                                        progress_status(
                                            &Some(tx.clone()),
                                            t("plugin_dev_progress_copying_files"),
                                        );
                                        for (idx, (rel_path_str, src_file_path)) in
                                            files.into_iter().enumerate()
                                        {
                                            progress_progress(
                                                &Some(tx.clone()),
                                                t("plugin_dev_progress_copying_file")
                                                    .replace("{}", &rel_path_str)
                                                    .replace("{n}", &(idx + 1).to_string())
                                                    .replace("{t}", &total.to_string()),
                                                idx + 1,
                                                total,
                                            );
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

                                        progress_status(
                                            &Some(tx.clone()),
                                            t("plugin_dev_progress_hashing_files"),
                                        );
                                        let mut files_hash: std::collections::HashMap<
                                            String,
                                            String,
                                        > = std::collections::HashMap::new();
                                        let dest_files =
                                            crate::plugin::loader::get_plugin_files(&dest_dir);
                                        let total_hash = dest_files.len().max(1);
                                        for (idx, (rel, p)) in dest_files.into_iter().enumerate() {
                                            progress_progress(
                                                &Some(tx.clone()),
                                                t("plugin_dev_progress_hashing_file")
                                                    .replace("{}", &rel)
                                                    .replace("{n}", &(idx + 1).to_string())
                                                    .replace("{t}", &total_hash.to_string()),
                                                idx + 1,
                                                total_hash,
                                            );
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
                                        let _ = crate::plugin::updater::write_lockfile(&lock);
                                        Ok(format!(
                                            "✓ Installed '{}' locally (copied {} file(s))\n\n{}",
                                            name,
                                            copied_files.len(),
                                            t("plugin_dev_local_sync_ok")
                                        ))
                                    })(
                                    );

                                    match res {
                                        Ok(msg) => {
                                            crate::plugin::developer_tool::progress_finish(
                                                Some(tx),
                                                Some(msg),
                                                None,
                                            );
                                        }
                                        Err(e) => {
                                            crate::plugin::developer_tool::progress_finish(
                                                Some(tx),
                                                None,
                                                Some(e),
                                            );
                                        }
                                    }
                                });
                            }
                        } else {
                            *dev_results = t("plugin_dev_no_active_err");
                        }
                    }
                    5 => {
                        // Submit Plugin
                        if let Some(plugin_folder) = active_plugin.clone() {
                            let path = if Path::new(&plugin_folder).is_absolute() {
                                PathBuf::from(&plugin_folder)
                            } else {
                                PathBuf::from(&plugins_dev_dir).join(&plugin_folder)
                            };
                            if !path.exists()
                                || !path.is_dir()
                                || !path.join("manifest.toml").exists()
                            {
                                *dev_results = format!(
                                    "Error: Plugin directory '{}' no longer exists.",
                                    plugin_folder
                                );
                            } else {
                                // First do a quick synchronous validation
                                // (it's cheap). If it passes, enter the
                                // wizard for commit message + token.
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
                            }
                        } else {
                            *dev_results = t("plugin_dev_no_active_err");
                        }
                    }
                    6 => {
                        // Open dev plugin folder in active panel
                        let target = dev_plugin_dir(context);
                        if !target.exists() {
                            *dev_results = t("plugin_dev_folder_not_found")
                                .replace("{:?}", &target.to_string_lossy());
                        } else {
                            move_active_panel_to(
                                state,
                                target,
                                context.config.settings.show_hidden,
                            );
                        }
                    }
                    7 => {
                        // Open package folder in active panel
                        if let Some(plugin_folder) = active_plugin.as_ref() {
                            let specific = packaged_plugin_dir(plugin_folder);
                            let target = if specific.as_ref().map(|p| p.exists()).unwrap_or(false) {
                                specific.unwrap()
                            } else {
                                let fallback =
                                    crate::config::paths::get_cache_dir().join("temp_registry");
                                if fallback.exists() {
                                    fallback
                                } else {
                                    *dev_results = t("plugin_dev_package_folder_missing");
                                    return;
                                }
                            };
                            move_active_panel_to(
                                state,
                                target,
                                context.config.settings.show_hidden,
                            );
                        } else {
                            *dev_results = t("plugin_dev_no_active_err");
                        }
                    }
                    8 => {
                        // Open submit folder in active panel
                        let target = crate::config::paths::get_cache_dir().join("temp_registry");
                        if !target.exists() {
                            *dev_results = t("plugin_dev_submit_folder_missing")
                                .replace("{:?}", &target.to_string_lossy());
                        } else {
                            move_active_panel_to(
                                state,
                                target,
                                context.config.settings.show_hidden,
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

// Suppress unused warning for the helper kept for future use.
#[allow(dead_code)]
fn _suppress_unused_is_init_option() {
    let _ = is_init_option(0);
}
