use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;

pub fn open_plugin_menu(state: &mut AppState, context: &AppContext) {
    // Open the popup immediately so the UI stays responsive while we
    // fetch the registry index and assemble the installed list in the
    // background. The status line + spinner shows progress to the user.
    state.dialogs.replace(PopupType::PluginMenu {
        active_tab: 0,
        cursor_idx: 0,
        installed: Vec::new(),
        all_registry: Vec::new(),
        registry: Vec::new(),
        search_query: String::new(),
        is_searching: false,
        editing_query: false,
        dev_results: String::new(),
        dev_wizard_step: 0,
        dev_wizard_data: Vec::new(),
        installed_loading: true,
        installed_loading_status: t("plugin_dev_progress_loading_index"),
        dev_loading: false,
        dev_loading_status: String::new(),
        dev_loading_progress: None,
    });

    // Snapshot the data we need from `context` (which is borrowed
    // mutably) so the background task does not capture a reference
    // to it.
    let plugins_settings = context.config.settings.plugins.clone();
    let tx = crate::plugin::PluginManager::get_sender();
    tokio::spawn(async move {
        let lock = crate::plugin::updater::read_lockfile();
        let index = crate::plugin::updater::fetch_index().await.ok();
        let mut installed = Vec::new();
        for (name, info) in &lock.plugins {
            let trusted = plugins_settings
                .get(name)
                .map(|p| p.trusted)
                .unwrap_or(false);

            let update_available = if let Some(ref idx) = index {
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
        // Build the full registry list (all available plugins) so the
        // Search tab shows results immediately without requiring a query.
        let registry: Vec<(String, String, String, String)> = index
            .as_ref()
            .map(|idx| {
                let mut list: Vec<_> = idx
                    .plugins
                    .iter()
                    .map(|(name, p)| {
                        (
                            name.clone(),
                            p.version.clone(),
                            p.description.clone().unwrap_or_default(),
                            p.author.clone().unwrap_or_default(),
                        )
                    })
                    .collect();
                list.sort_by(|a, b| a.0.cmp(&b.0));
                list
            })
            .unwrap_or_default();
        let _ = tx
            .send(crate::plugin::manager::PluginRequest::PluginMenuLoaded {
                installed,
                registry,
            })
            .await;
    });
}

pub fn install_dev_plugin(state: &mut AppState, context: &AppContext) -> bool {
    if !context.config.settings.plugins_developer_mode {
        return false;
    }
    let active_panel = state.get_active_panel();
    let current_dir = &active_panel.current_path;

    let mut target_dir = current_dir.clone();
    if let Some(entry) = active_panel.entries.get(active_panel.cursor_index)
        && entry.path.is_dir()
        && entry.path.join("manifest.toml").exists()
    {
        target_dir = entry.path.clone();
    }

    let manifest_path = target_dir.join("manifest.toml");
    if manifest_path.exists()
        && let Ok(manifest_content) = std::fs::read_to_string(&manifest_path)
        && let Ok(manifest) = crate::plugin::loader::PluginManifest::parse(&manifest_content)
    {
        let name = manifest.name.clone();
        let version = manifest.version.clone();
        let dest_dir = crate::config::paths::get_config_dir()
            .join("plugins")
            .join(format!("{}.pairee", name));

        let _ = std::fs::create_dir_all(&dest_dir);
        let mut success = true;
        if let Ok(entries) = std::fs::read_dir(&target_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(filename) = path.file_name() {
                        let _ = std::fs::copy(&path, dest_dir.join(filename));
                    }
                } else if path.is_dir() && path.file_name().map(|n| n == "lang").unwrap_or(false) {
                    let lang_dest = dest_dir.join("lang");
                    let _ = std::fs::create_dir_all(&lang_dest);
                    if let Ok(lang_entries) = std::fs::read_dir(&path) {
                        for le in lang_entries.filter_map(Result::ok) {
                            if le.path().is_file()
                                && let Some(fn_lang) = le.path().file_name()
                            {
                                let _ = std::fs::copy(le.path(), lang_dest.join(fn_lang));
                            }
                        }
                    }
                }
            }
        } else {
            success = false;
        }

        if success {
            let mut lock = crate::plugin::updater::read_lockfile();
            let mut files_hash = std::collections::HashMap::new();
            for (rel, p) in crate::plugin::loader::get_plugin_files(&dest_dir) {
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

            state.dialogs.replace(crate::app::state::PopupType::Info(
                t("plugin_toast_install_dev_ok")
                    .replace("{}", &name)
                    .replace("{:?}", &format!("{:?}", dest_dir)),
            ));
        } else {
            state.dialogs.replace(crate::app::state::PopupType::Error(
                t("plugin_toast_install_dev_failed").replace("{}", &name),
            ));
        }
    }
    true
}
