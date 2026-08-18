use super::move_active_panel_to;
use super::paths::{dev_plugin_dir, packaged_plugin_dir};
use super::progress::{begin_dev_op, progress_status};
use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::config::localization::t;
use crate::plugin::developer_tool;

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_option_select_active_plugin(
    context: &mut AppContext,
    dev_results: &mut String,
    installed: &mut Vec<(String, String, bool, bool, Option<String>)>,
    left_panel_path: &std::path::Path,
    right_panel_path: &std::path::Path,
    plugins_dev_dir: std::path::PathBuf,
) {
    // === Option 0: Select / Change / Deselect active plugin ===
    // The scan runs on the blocking pool to keep the UI
    // responsive even when the dev folder is large.
    *dev_results = t("plugin_dev_progress_scanning_plugins");
    let left = left_panel_path.to_path_buf();
    let right = right_panel_path.to_path_buf();
    let tx = crate::plugin::PluginManager::get_sender();
    let plugins_dev_dir_for_task = plugins_dev_dir.clone();
    tokio::task::spawn_blocking(move || {
        let mut options = Vec::new();
        options.push((t("plugin_dev_deselect_option"), "deselect".to_string()));

        if let Ok(entries) = std::fs::read_dir(&plugins_dev_dir_for_task) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir()
                    && path.join("manifest.toml").exists()
                    && let Some(name) = path.file_name().and_then(|n| n.to_str())
                {
                    options.push((name.to_string(), name.to_string()));
                }
            }
        }

        if left.join("manifest.toml").exists()
            && let Some(name) = left.file_name().and_then(|n| n.to_str())
        {
            options.push((
                t("plugin_dev_panel1").replacen("{}", name, 1).replacen(
                    "{}",
                    &left.display().to_string(),
                    1,
                ),
                left.to_string_lossy().to_string(),
            ));
        }

        if right.join("manifest.toml").exists()
            && let Some(name) = right.file_name().and_then(|n| n.to_str())
        {
            options.push((
                t("plugin_dev_panel2").replacen("{}", name, 1).replacen(
                    "{}",
                    &right.display().to_string(),
                    1,
                ),
                right.to_string_lossy().to_string(),
            ));
        }

        let _ = tx.blocking_send(crate::plugin::manager::PluginRequest::DevPluginScan { options });
    });
    *installed = super::reload_installed_plugins(context, &None);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_option_init_plugin(
    _context: &mut AppContext,
    active_plugin: Option<String>,
    editing_query: &mut bool,
    search_query: &mut String,
    dev_results: &mut String,
    dev_wizard_step: &mut usize,
    dev_wizard_data: &mut Vec<String>,
) {
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

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_option_lint(
    state: &mut AppState,
    _context: &mut AppContext,
    dev_results: &mut String,
    active_plugin: Option<String>,
    plugins_dev_dir: std::path::PathBuf,
) {
    // Lint active plugin (async)
    if let Some(plugin_folder) = active_plugin.clone() {
        let path = resolve_active_plugin_path(&plugin_folder, &plugins_dev_dir);
        if !path.exists() || !path.is_dir() || !path.join("manifest.toml").exists() {
            *dev_results = t("plugin_dev_dir_missing").replace("{}", &plugin_folder);
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
                let res = developer_tool::lint_with_progress(Some(tx.clone()));
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
                        developer_tool::progress_finish(Some(tx), Some(result), None);
                    }
                    Err(e) => {
                        developer_tool::progress_finish(Some(tx), None, Some(format!("{:?}", e)));
                    }
                }
            });
        }
    } else {
        *dev_results = t("plugin_dev_no_active_err");
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_option_package(
    state: &mut AppState,
    _context: &mut AppContext,
    dev_results: &mut String,
    active_plugin: Option<String>,
    plugins_dev_dir: std::path::PathBuf,
) {
    // Package active plugin (async)
    if let Some(plugin_folder) = active_plugin.clone() {
        let path = resolve_active_plugin_path(&plugin_folder, &plugins_dev_dir);
        if !path.exists() || !path.is_dir() || !path.join("manifest.toml").exists() {
            *dev_results = t("plugin_dev_dir_missing").replace("{}", &plugin_folder);
        } else {
            *dev_results = t("plugin_dev_pack_start")
                .replace("{}", &plugin_folder)
                .trim()
                .to_string();
            let tx = begin_dev_op(state, t("plugin_dev_progress_fetching_registry"));
            let path_for_task = path.clone();
            let name_for_result = plugin_folder.clone();
            tokio::task::spawn_blocking(move || {
                match developer_tool::package_to_registry_with_progress(
                    &path_for_task,
                    Some(tx.clone()),
                ) {
                    Ok(msg) => {
                        let result = format!(
                            "✓ {}\n\n{}",
                            msg,
                            t("plugin_dev_pack_done_tail").replace("{}", &name_for_result)
                        );
                        developer_tool::progress_finish(Some(tx), Some(result), None);
                    }
                    Err(e) => {
                        developer_tool::progress_finish(
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

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_option_install_local(
    state: &mut AppState,
    _context: &mut AppContext,
    dev_results: &mut String,
    active_plugin: Option<String>,
    plugins_dev_dir: std::path::PathBuf,
) {
    // Install active plugin locally (async with per-file progress)
    if let Some(plugin_folder) = active_plugin.clone() {
        let path = resolve_active_plugin_path(&plugin_folder, &plugins_dev_dir);
        if !path.exists() || !path.is_dir() || !path.join("manifest.toml").exists() {
            *dev_results = t("plugin_dev_dir_missing").replace("{}", &plugin_folder);
        } else {
            *dev_results = format!("{} '{}'…", t("plugin_dev_install_start"), plugin_folder);
            let tx = begin_dev_op(state, t("plugin_dev_progress_copying_files"));
            let path_for_task = path.clone();
            tokio::task::spawn_blocking(move || {
                use developer_tool::progress_progress;
                let manifest_path = path_for_task.join("manifest.toml");
                let res: Result<String, String> = (|| {
                    let content =
                        std::fs::read_to_string(&manifest_path).map_err(|e| format!("{:?}", e))?;
                    let manifest = crate::plugin::loader::PluginManifest::parse(&content)
                        .map_err(|e| format!("{:?}", e))?;
                    let name = manifest.name.clone();
                    let version = manifest.version.clone();
                    let dest_base = crate::config::paths::get_config_dir().join("plugins");
                    let mut lock = crate::plugin::updater::read_lockfile();
                    let dest_dir = dest_base.join(format!("{}.pairee", name));
                    let _ = std::fs::create_dir_all(&dest_dir);

                    let files = crate::plugin::loader::get_plugin_files(&path_for_task);
                    let total = files.len().max(1);
                    let mut copied_files = Vec::new();
                    progress_status(&Some(tx.clone()), t("plugin_dev_progress_copying_files"));
                    for (idx, (rel_path_str, src_file_path)) in files.into_iter().enumerate() {
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
                        if std::fs::copy(&src_file_path, &dest_file_path).is_ok() {
                            copied_files.push(rel_path_str);
                        }
                    }

                    progress_status(&Some(tx.clone()), t("plugin_dev_progress_hashing_files"));
                    let mut files_hash: std::collections::HashMap<String, String> =
                        std::collections::HashMap::new();
                    let dest_files = crate::plugin::loader::get_plugin_files(&dest_dir);
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
                        if let Ok(h) = crate::update::downloader::compute_sha256(&p) {
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
                    Ok(t("plugin_dev_installed_locally")
                        .replacen("{}", &name, 1)
                        .replacen("{}", &copied_files.len().to_string(), 1)
                        .replacen("{}", &t("plugin_dev_local_sync_ok"), 1))
                })();

                match res {
                    Ok(msg) => {
                        developer_tool::progress_finish(Some(tx), Some(msg), None);
                    }
                    Err(e) => {
                        developer_tool::progress_finish(Some(tx), None, Some(e));
                    }
                }

                // The installed list is reloaded by the caller
                // (`handle_option_install_local`) after the spawning
                // completes, so no explicit reload is needed here.
            });
        }
    } else {
        *dev_results = t("plugin_dev_no_active_err");
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_option_submit(
    _context: &mut AppContext,
    dev_results: &mut String,
    active_plugin: Option<String>,
    plugins_dev_dir: std::path::PathBuf,
    editing_query: &mut bool,
    search_query: &mut String,
    dev_wizard_step: &mut usize,
    dev_wizard_data: &mut Vec<String>,
) {
    // Submit Plugin
    if let Some(plugin_folder) = active_plugin.clone() {
        let path = resolve_active_plugin_path(&plugin_folder, &plugins_dev_dir);
        if !path.exists() || !path.is_dir() || !path.join("manifest.toml").exists() {
            *dev_results = t("plugin_dev_dir_missing").replace("{}", &plugin_folder);
        } else {
            // First do a quick synchronous validation
            // (it's cheap). If it passes, enter the
            // wizard for commit message + token.
            match developer_tool::validate_for_publish(&path) {
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

pub(super) fn handle_option_open_dev_folder(
    state: &mut AppState,
    context: &AppContext,
    dev_results: &mut String,
) {
    // Open dev plugin folder in active panel
    let target = dev_plugin_dir(context);
    if !target.exists() {
        *dev_results = t("plugin_dev_folder_not_found").replace("{:?}", &target.to_string_lossy());
    } else {
        move_active_panel_to(state, target, context.config.settings.show_hidden);
    }
}

pub(super) fn handle_option_open_package_folder(
    state: &mut AppState,
    context: &AppContext,
    dev_results: &mut String,
    active_plugin: Option<String>,
) {
    // Open package folder in active panel
    if let Some(plugin_folder) = active_plugin.as_ref() {
        let specific = packaged_plugin_dir(plugin_folder);
        let target = if specific.as_ref().map(|p| p.exists()).unwrap_or(false) {
            specific.unwrap()
        } else {
            let fallback = crate::config::paths::get_cache_dir().join("temp_registry");
            if fallback.exists() {
                fallback
            } else {
                *dev_results = t("plugin_dev_package_folder_missing");
                return;
            }
        };
        move_active_panel_to(state, target, context.config.settings.show_hidden);
    } else {
        *dev_results = t("plugin_dev_no_active_err");
    }
}

pub(super) fn handle_option_open_submit_folder(
    state: &mut AppState,
    context: &AppContext,
    dev_results: &mut String,
) {
    // Open submit folder in active panel
    let target = crate::config::paths::get_cache_dir().join("temp_registry");
    if !target.exists() {
        *dev_results =
            t("plugin_dev_submit_folder_missing").replace("{:?}", &target.to_string_lossy());
    } else {
        move_active_panel_to(state, target, context.config.settings.show_hidden);
    }
}

pub(super) fn resolve_active_plugin_path(
    plugin_folder: &str,
    plugins_dev_dir: &std::path::Path,
) -> std::path::PathBuf {
    if std::path::Path::new(plugin_folder).is_absolute() {
        std::path::PathBuf::from(plugin_folder)
    } else {
        plugins_dev_dir.join(plugin_folder)
    }
}
