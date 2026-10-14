use super::super::move_active_panel_to;
use super::super::paths::{dev_plugin_dir, packaged_plugin_dir};
use crate::app::context::AppContext;
use crate::app::state::{AppState, PluginMenuState};
use crate::config::localization::t;

pub fn handle_option_select_active_plugin(
    context: &mut AppContext,
    menu: &mut PluginMenuState,
    (left_panel_path, right_panel_path): (&std::path::Path, &std::path::Path),
    plugins_dev_dir: std::path::PathBuf,
) {
    menu.dev_results = t("plugin_dev_progress_scanning_plugins");
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
    menu.installed = super::super::reload_installed_plugins(context, &None);
}

pub fn handle_option_open_dev_folder(
    state: &mut AppState,
    context: &AppContext,
    dev_results: &mut String,
) {
    let target = dev_plugin_dir(context);
    if !target.exists() {
        *dev_results = t("plugin_dev_folder_not_found").replace("{:?}", &target.to_string_lossy());
    } else {
        move_active_panel_to(state, target, context.config.settings.show_hidden);
    }
}

pub fn handle_option_open_package_folder(
    state: &mut AppState,
    context: &AppContext,
    dev_results: &mut String,
    active_plugin: Option<String>,
) {
    if active_plugin.is_some() {
        // The active plugin's name is relative to the development folder
        // (not to the current directory): resolve it like option 6 does.
        let specific = packaged_plugin_dir(&dev_plugin_dir(context));
        let target = if let Some(specific) = specific.filter(|p| p.exists()) {
            specific
        } else {
            let fallback = crate::plugin::developer_tool::temp_registry_dir();
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

pub fn handle_option_open_submit_folder(
    state: &mut AppState,
    context: &AppContext,
    dev_results: &mut String,
) {
    let target = crate::plugin::developer_tool::temp_registry_dir();
    if !target.exists() {
        *dev_results =
            t("plugin_dev_submit_folder_missing").replace("{:?}", &target.to_string_lossy());
    } else {
        move_active_panel_to(state, target, context.config.settings.show_hidden);
    }
}
