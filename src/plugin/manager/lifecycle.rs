//! Plugin manager lifecycle: channel initialization, loading the enabled
//! plugins at startup and reloading them when the user changes Secure
//! Mode, a plugin's trust or whether it is enabled.
//!
//! All cross-thread communication with the main loop goes through the
//! `mpsc` channels declared in this file. The dispatcher (in
//! `dispatcher.rs`) drains the receiver on every main-loop tick.

use crate::config::AppConfig;
use crate::plugin::load_options::{PlannedPlugin, load_plan};
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
        tracing::info!("PluginManager initialized request channels.");
    }

    pub fn get_sender() -> mpsc::Sender<PluginRequest> {
        PLUGIN_REQ_TX
            .get()
            .cloned()
            .expect("PluginManager channels not initialized")
    }

    /// The enabled plugins of the plugins folder, with the options
    /// `config` gives them (creates the folder when missing).
    fn plan(config: &AppConfig) -> Vec<PlannedPlugin> {
        let plugins_dir = crate::config::paths::get_config_dir().join("plugins");
        if !plugins_dir.exists() {
            let _ = std::fs::create_dir_all(&plugins_dir);
        }
        load_plan(config, &plugins_dir)
    }

    /// Loads every enabled plugin (each in its own task).
    pub async fn load_all_plugins(config: &AppConfig) {
        for planned in Self::plan(config) {
            spawn_load(planned);
        }
    }

    /// Unloads every plugin and loads the enabled ones again with the
    /// options `config` gives them now, so a change of Secure Mode, trust
    /// or enabled state applies to the running session. Returns at once.
    pub fn reload_all(config: &AppConfig) {
        if PLUGIN_REQ_TX.get().is_none() {
            return;
        }
        let plan = Self::plan(config);
        tokio::spawn(async move {
            for info in crate::plugin::registry::get_loaded_plugins().await {
                crate::plugin::registry::unregister_plugin(&info.manifest.name).await;
            }
            for planned in plan {
                spawn_load(planned);
            }
        });
    }
}

fn spawn_load(planned: PlannedPlugin) {
    let tx = PluginManager::get_sender();
    tokio::spawn(async move {
        let name = planned.options.name.clone();
        tracing::info!("Loading plugin {} from {:?}", name, planned.dir);
        if let Err(e) = crate::plugin::loader::load_plugin(&planned.dir, &planned.options, tx).await
        {
            tracing::error!("Failed to load plugin {}: {:?}", name, e);
        }
    });
}
