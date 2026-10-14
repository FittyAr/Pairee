use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::config::localization::t;

pub fn open_plugin_menu(state: &mut AppState, context: &AppContext) {
    // Open the popup immediately so the UI stays responsive while we
    // fetch the registry index and assemble the installed list in the
    // background. The status line + spinner shows progress to the user.
    state.dialogs.replace(crate::app::state::PluginMenuState {
        installed_loading: true,
        installed_loading_status: t("plugin_dev_progress_loading_index"),
        ..Default::default()
    });

    // Snapshot the data we need from `context` (which is borrowed
    // mutably) so the background task does not capture a reference
    // to it.
    let plugins_settings = context.config.settings.plugins.clone();
    let secure_mode = crate::plugin::load_options::effective_secure_mode(&context.config);
    let trust = crate::plugin::updater::RegistryTrust::from_settings(&context.config.settings);
    let tx = crate::plugin::PluginManager::get_sender();
    tokio::spawn(async move {
        let lock = crate::plugin::updater::read_lockfile();
        let index = crate::plugin::updater::fetch_index(&trust).await.ok();
        let installed = crate::plugin::installed::installed_rows(
            &lock,
            &plugins_settings,
            secure_mode,
            index.as_ref(),
        );
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

    if !target_dir.join("manifest.toml").exists() {
        return true;
    }
    let popup = match crate::app::plugin_install::install_local(&target_dir, &|_, _, _| {}) {
        Ok(done) => crate::app::state::PopupType::info(
            t("plugin_toast_install_dev_ok")
                .replace("{}", &done.name)
                .replace("{:?}", &format!("{:?}", done.dest)),
        ),
        Err(error) => crate::app::state::PopupType::error(format!(
            "{}\n{error}",
            t("plugin_toast_install_dev_failed").replace("{}", &target_dir.to_string_lossy())
        )),
    };
    state.dialogs.replace(popup);
    true
}
