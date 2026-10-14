//! The loaded plugins: their manifests, the methods they define and the
//! channel to each plugin's worker task (see [`super::worker`]), plus the
//! host-side entry points that send work to them (`run_previewer`,
//! `run_preload`, `run_seek`, `run_command`, `emit_hook_event`).

use crate::app::state::types::Renderable;
use crate::plugin::annotations::Annotations;
use crate::plugin::loader::PluginManifest;
use crate::plugin::manager::{PluginOrigin, PluginRequest};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::sync::{RwLock, mpsc, oneshot};

/// What a previewer is asked to show: the file, the size of the preview
/// area (in cells) and how far the user scrolled (`skip`, in lines).
#[derive(Debug, Clone)]
pub struct PreviewJob {
    pub file_path: PathBuf,
    pub area_width: u16,
    pub area_height: u16,
    pub skip: usize,
}

pub enum PluginTaskRequest {
    Peek {
        job: PreviewJob,
        reply_tx: oneshot::Sender<Option<Renderable>>,
    },
    /// `preload(job)` ahead of a peek; replies whether the plugin reported
    /// the work complete.
    Preload {
        job: PreviewJob,
        reply_tx: oneshot::Sender<bool>,
    },
    /// `seek(job)` with `job.units` lines to scroll; replies the new `skip`
    /// the plugin chose (`None`: no `seek` or no number returned).
    Seek {
        job: PreviewJob,
        units: i64,
        reply_tx: oneshot::Sender<Option<usize>>,
    },
    ExecuteCommand {
        args: Vec<String>,
    },
    EmitEvent {
        name: String,
        data: String, // JSON payload
        /// The plugin that published it (`None`: a host event).
        sender: Option<PluginOrigin>,
    },
}

/// The plugin methods the host may call, found when the plugin loaded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PluginMethods {
    pub peek: bool,
    pub preload: bool,
    pub seek: bool,
    pub entry: bool,
}

impl PluginMethods {
    /// Which methods the plugin table `M` defines.
    pub fn of(table: &mlua::Table) -> Self {
        let has =
            |name: &str| matches!(table.get::<mlua::Value>(name), Ok(mlua::Value::Function(_)));
        Self {
            peek: has("peek"),
            preload: has("preload"),
            seek: has("seek"),
            entry: has("entry"),
        }
    }
}

#[derive(Clone)]
pub struct PluginInfo {
    pub manifest: PluginManifest,
    pub path: PathBuf,
    pub methods: PluginMethods,
}

struct Registry {
    plugins: RwLock<HashMap<String, PluginInfo>>,
    channels: RwLock<HashMap<String, mpsc::Sender<PluginTaskRequest>>>,
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

fn get_registry() -> &'static Registry {
    REGISTRY.get_or_init(|| Registry {
        plugins: RwLock::new(HashMap::new()),
        channels: RwLock::new(HashMap::new()),
    })
}

/// A loaded plugin, ready to be registered.
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub path: PathBuf,
    pub lua: mlua::Lua,
    pub table_key: mlua::RegistryKey,
    pub annotations: Annotations,
    /// Which methods its table defines (read under the watchdog).
    pub methods: PluginMethods,
}

/// Registers a loaded plugin and starts its worker task. `tx` is the
/// plugin's channel to the main loop (for `--- @sync` refreshes).
pub async fn register_plugin(
    plugin: LoadedPlugin,
    tx: mpsc::Sender<PluginRequest>,
) -> anyhow::Result<()> {
    let name = plugin.manifest.name.clone();
    let registry = get_registry();
    let info = PluginInfo {
        manifest: plugin.manifest.clone(),
        path: plugin.path.clone(),
        methods: plugin.methods,
    };
    registry.plugins.write().await.insert(name.clone(), info);

    // Its commands join the keymap (replacing those of an earlier load).
    crate::keybindings::plugin_commands::register(&name, plugin.manifest.keymap_commands());

    let (task_tx, rx) = mpsc::channel::<PluginTaskRequest>(50);
    registry
        .channels
        .write()
        .await
        .insert(name.clone(), task_tx);
    super::worker::spawn(
        super::worker::Worker::new(name, plugin.lua, plugin.table_key, plugin.annotations, tx),
        rx,
    );
    Ok(())
}

/// Sends the request `make` builds to plugin `name` and awaits its reply
/// (`None` when the plugin is not loaded or its task ended).
async fn ask<T>(
    name: &str,
    make: impl FnOnce(oneshot::Sender<T>) -> PluginTaskRequest,
) -> Option<T> {
    let tx = get_registry().channels.read().await.get(name).cloned()?;
    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(make(reply_tx)).await.ok()?;
    reply_rx.await.ok()
}

/// Sends a request that has no reply to plugin `name`.
async fn tell(name: &str, req: PluginTaskRequest) {
    let tx = get_registry().channels.read().await.get(name).cloned();
    if let Some(tx) = tx {
        let _ = tx.send(req).await;
    }
}

/// Like [`tell`], but drops the request when the plugin's queue is full.
async fn try_tell(name: &str, req: PluginTaskRequest) {
    let tx = get_registry().channels.read().await.get(name).cloned();
    if let Some(tx) = tx
        && let Err(mpsc::error::TrySendError::Full(_)) = tx.try_send(req)
    {
        tracing::debug!("Plugin {name} is busy; an event was dropped");
    }
}

pub async fn run_previewer(name: &str, job: PreviewJob) -> Option<Renderable> {
    ask(name, |reply_tx| PluginTaskRequest::Peek { job, reply_tx })
        .await
        .flatten()
}

/// Runs plugin `name`'s `preload(job)`; `true` when it reported the work
/// complete (or had already completed it for this file version).
pub async fn run_preload(name: &str, job: PreviewJob) -> bool {
    ask(name, |reply_tx| PluginTaskRequest::Preload {
        job,
        reply_tx,
    })
    .await
    .unwrap_or(false)
}

/// Runs plugin `name`'s `seek(job)` with `units` lines to scroll.
pub async fn run_seek(name: &str, job: PreviewJob, units: i64) -> Option<usize> {
    ask(name, |reply_tx| PluginTaskRequest::Seek {
        job,
        units,
        reply_tx,
    })
    .await
    .flatten()
}

pub async fn run_command(name: &str, args: Vec<String>) {
    tell(name, PluginTaskRequest::ExecuteCommand { args }).await;
}

pub async fn emit_hook_event(
    plugin_name: &str,
    event_name: &str,
    data: String,
    sender: Option<PluginOrigin>,
) {
    let req = PluginTaskRequest::EmitEvent {
        name: event_name.to_string(),
        data,
        sender,
    };
    try_tell(plugin_name, req).await;
}

pub async fn get_loaded_plugins() -> Vec<PluginInfo> {
    let registry = get_registry();
    registry.plugins.read().await.values().cloned().collect()
}

/// The loaded plugin `name`, if any.
pub async fn plugin_info(name: &str) -> Option<PluginInfo> {
    get_registry().plugins.read().await.get(name).cloned()
}

/// Forgets a plugin that was uninstalled: its commands, its entry in the
/// loaded list and its channel (which ends its task).
pub async fn unregister_plugin(name: &str) {
    let registry = get_registry();
    crate::keybindings::plugin_commands::unregister(name);
    registry.plugins.write().await.remove(name);
    registry.channels.write().await.remove(name);
}
