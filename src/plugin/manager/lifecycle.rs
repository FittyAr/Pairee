//! Plugin manager lifecycle: channel initialization and eager plugin
//! discovery at startup.
//!
//! All cross-thread communication with the main loop goes through the
//! `mpsc` channels declared in this file. The dispatcher (in
//! `dispatcher.rs`) drains the receiver on every main-loop tick.

use crate::app::context::AppContext;
use std::sync::OnceLock;
use tokio::sync::{Mutex, mpsc};

use super::request::PluginRequest;

pub static PLUGIN_REQ_TX: OnceLock<mpsc::Sender<PluginRequest>> = OnceLock::new();
pub static PLUGIN_REQ_RX: OnceLock<Mutex<mpsc::Receiver<PluginRequest>>> = OnceLock::new();

pub struct PluginManager;

impl PluginManager {
    pub fn init() {
        let (tx, rx) = mpsc::channel(100);
        let _ = PLUGIN_REQ_TX.set(tx);
        let _ = PLUGIN_REQ_RX.set(Mutex::new(rx));
        log::info!("PluginManager initialized request channels.");
    }

    pub fn get_sender() -> mpsc::Sender<PluginRequest> {
        PLUGIN_REQ_TX
            .get()
            .cloned()
            .expect("PluginManager channels not initialized")
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
