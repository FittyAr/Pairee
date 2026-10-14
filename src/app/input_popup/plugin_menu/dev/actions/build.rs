use super::super::progress::{begin_dev_op, progress_status};
use crate::app::context::AppContext;
use crate::app::state::{AppState, PluginMenuState};
use crate::config::localization::t;
use crate::plugin::developer_tool;

/// Starts the question / answer wizard at `step` with `data` collected so far.
fn start_wizard(menu: &mut PluginMenuState, step: usize, data: Vec<String>) {
    menu.editing_query = true;
    menu.search_query.clear();
    menu.dev_results.clear();
    menu.dev_wizard_step = step;
    menu.dev_wizard_data = data;
}

pub fn handle_option_init_plugin(active_plugin: Option<String>, menu: &mut PluginMenuState) {
    if active_plugin.is_some() {
        menu.dev_results = t("plugin_dev_desc_init_disabled");
    } else {
        start_wizard(menu, 1, Vec::new());
    }
}

pub fn handle_option_lint(
    state: &mut AppState,
    _context: &mut AppContext,
    dev_results: &mut String,
    active_plugin: Option<String>,
    plugins_dev_dir: std::path::PathBuf,
) {
    if let Some(plugin_folder) = active_plugin.clone() {
        let path = super::resolve_active_plugin_path(&plugin_folder, &plugins_dev_dir);
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
                let res = developer_tool::lint_with_progress(&path_for_task, Some(tx.clone()));
                match res {
                    Ok(details) => {
                        let result = format!(
                            "{details}\n\n{} '{}' {}",
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

pub fn handle_option_package(
    state: &mut AppState,
    _context: &mut AppContext,
    dev_results: &mut String,
    active_plugin: Option<String>,
    plugins_dev_dir: std::path::PathBuf,
) {
    if let Some(plugin_folder) = active_plugin.clone() {
        let path = super::resolve_active_plugin_path(&plugin_folder, &plugins_dev_dir);
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

pub fn handle_option_install_local(
    state: &mut AppState,
    _context: &mut AppContext,
    dev_results: &mut String,
    active_plugin: Option<String>,
    plugins_dev_dir: std::path::PathBuf,
) {
    if let Some(plugin_folder) = active_plugin.clone() {
        let path = super::resolve_active_plugin_path(&plugin_folder, &plugins_dev_dir);
        if !path.exists() || !path.is_dir() || !path.join("manifest.toml").exists() {
            *dev_results = t("plugin_dev_dir_missing").replace("{}", &plugin_folder);
        } else {
            *dev_results = format!("{} '{}'…", t("plugin_dev_install_start"), plugin_folder);
            let tx = begin_dev_op(state, t("plugin_dev_progress_copying_files"));
            let path_for_task = path.clone();
            tokio::task::spawn_blocking(move || {
                use crate::app::plugin_install::{InstallStep, install_local};
                progress_status(&Some(tx.clone()), t("plugin_dev_progress_copying_files"));
                let report = |step: InstallStep<'_>, n: usize, total: usize| {
                    let (key, file) = match step {
                        InstallStep::Copying(file) => ("plugin_dev_progress_copying_file", file),
                        InstallStep::Hashing(file) => {
                            if n == 1 {
                                let status = t("plugin_dev_progress_hashing_files");
                                progress_status(&Some(tx.clone()), status);
                            }
                            ("plugin_dev_progress_hashing_file", file)
                        }
                    };
                    developer_tool::progress_progress(
                        &Some(tx.clone()),
                        t(key)
                            .replace("{}", file)
                            .replace("{n}", &n.to_string())
                            .replace("{t}", &total.to_string()),
                        n,
                        total,
                    );
                };
                let res = install_local(&path_for_task, &report).map(|done| {
                    t("plugin_dev_installed_locally")
                        .replacen("{}", &done.name, 1)
                        .replacen("{}", &done.files.to_string(), 1)
                        .replacen("{}", &t("plugin_dev_local_sync_ok"), 1)
                });

                match res {
                    Ok(msg) => {
                        developer_tool::progress_finish(Some(tx), Some(msg), None);
                    }
                    Err(e) => {
                        developer_tool::progress_finish(Some(tx), None, Some(e));
                    }
                }
            });
        }
    } else {
        *dev_results = t("plugin_dev_no_active_err");
    }
}

pub fn handle_option_submit(
    menu: &mut PluginMenuState,
    active_plugin: Option<String>,
    plugins_dev_dir: std::path::PathBuf,
) {
    if let Some(plugin_folder) = active_plugin.clone() {
        let path = super::resolve_active_plugin_path(&plugin_folder, &plugins_dev_dir);
        if !path.exists() || !path.is_dir() || !path.join("manifest.toml").exists() {
            menu.dev_results = t("plugin_dev_dir_missing").replace("{}", &plugin_folder);
        } else {
            match developer_tool::validate_for_publish(&path) {
                Ok(_) => start_wizard(menu, 5, vec![path.to_string_lossy().to_string()]),
                Err(err_msg) => menu.dev_results = err_msg,
            }
        }
    } else {
        menu.dev_results = t("plugin_dev_no_active_err");
    }
}
