//! Plugin manager lifecycle and eager plugin discovery at
//! startup.
//!
//! All cross-thread communication with the main loop goes through
//! the plugin-request `mpsc` channel. The **receiver** is owned
//! by `AppState` (see `app::state::AppState::new`) so the
//! dispatcher can `try_recv` without a global lock; the
//! **sender** is exposed through the `PLUGIN_REQ_TX` `OnceLock`
//! so the plugin runtime and other call sites that don't have
//! a state handle can publish requests.
//!
//! Note: the channel is created lazily on the first
//! `AppState::new`. Callers that grab a sender before any state
//! exists will see a panic from `get_sender`; in practice
//! `AppState::new` is called early in `main`, well before any
//! plugin code runs.

use crate::app::context::AppContext;
use std::sync::OnceLock;
use tokio::sync::mpsc;

use super::request::PluginRequest;

pub static PLUGIN_REQ_TX: OnceLock<mpsc::Sender<PluginRequest>> = OnceLock::new();

pub struct PluginManager;

impl PluginManager {
    /// Return a clone of the global plugin request sender.
    /// Panics if `AppState::new` has not been called yet
    /// (which means the channel has not been created).
    pub fn get_sender() -> mpsc::Sender<PluginRequest> {
        PLUGIN_REQ_TX
            .get()
            .cloned()
            .expect("AppState must be constructed before calling PluginManager::get_sender")
    }

    pub async fn load_all_plugins(context: &AppContext) {
        let plugins_dir = crate::config::paths::get_config_dir().join("plugins");
        if !plugins_dir.exists() {
            let _ = std::fs::create_dir_all(&plugins_dir);
        }

        // Search directory for plugins
        if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let folder_name = path.file_name().unwrap().to_string_lossy().to_string();
                    if !folder_name.ends_with(".pairee") {
                        continue;
                    }
                    let name = folder_name.strip_suffix(".pairee").unwrap().to_string();
                    let enabled = context
                        .config
                        .settings
                        .plugins
                        .get(&name)
                        .map(|c| c.name == name)
                        .unwrap_or(true); // Enabled by default if not set otherwise

                    let trusted = context
                        .config
                        .settings
                        .plugins
                        .get(&name)
                        .map(|c| c.trusted)
                        .unwrap_or(false);

                    if enabled {
                        let tx = Self::get_sender();
                        let name_clone = name.clone();
                        let path_clone = path.clone();
                        tokio::spawn(async move {
                            log::info!("Loading plugin {} from {:?}", name_clone, path_clone);
                            if let Err(e) = crate::plugin::loader::load_plugin(
                                &name_clone,
                                &path_clone,
                                trusted,
                                tx,
                            )
                            .await
                            {
                                log::error!("Failed to load plugin {}: {:?}", name_clone, e);
                            }
                        });
                    }
                }
            }
        }
    }
}
