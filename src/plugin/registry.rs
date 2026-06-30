use crate::app::state::types::PluginWidget;
use crate::plugin::loader::PluginManifest;
use mlua::LuaSerdeExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::sync::{RwLock, mpsc, oneshot};

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
        reply_tx: oneshot::Sender<Option<PluginWidget>>,
    },
    ExecuteCommand {
        args: Vec<String>,
    },
    EmitEvent {
        name: String,
        data: String, // JSON payload
    },
}

#[derive(Clone)]
pub struct PluginInfo {
    pub manifest: PluginManifest,
    pub path: PathBuf,
}

struct Registry {
    plugins: RwLock<HashMap<String, PluginInfo>>,
    channels: RwLock<HashMap<String, mpsc::Sender<PluginTaskRequest>>>,
    keybindings: RwLock<HashMap<String, (String, String)>>, // Key -> (PluginName, ActionName)
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

fn get_registry() -> &'static Registry {
    REGISTRY.get_or_init(|| Registry {
        plugins: RwLock::new(HashMap::new()),
        channels: RwLock::new(HashMap::new()),
        keybindings: RwLock::new(HashMap::new()),
    })
}

pub async fn register_plugin(
    manifest: PluginManifest,
    table_key: mlua::RegistryKey,
    lua: mlua::Lua,
    path: PathBuf,
) -> anyhow::Result<()> {
    let name = manifest.name.clone();
    let registry = get_registry();

    // Insert into plugins list
    let info = PluginInfo {
        manifest: manifest.clone(),
        path: path.clone(),
    };
    registry.plugins.write().await.insert(name.clone(), info);

    // Register keybindings
    if let Some(ref keymaps) = manifest.keybindings {
        let mut keybindings = registry.keybindings.write().await;
        for (key, action) in keymaps {
            keybindings.insert(key.clone(), (name.clone(), action.clone()));
        }
    }

    // Set up communication channel and spawn task
    let (tx, mut rx) = mpsc::channel::<PluginTaskRequest>(50);
    registry.channels.write().await.insert(name.clone(), tx);

    tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            match req {
                PluginTaskRequest::Peek { job, reply_tx } => {
                    let res = execute_peek_internal(&lua, &table_key, job);
                    let _ = reply_tx.send(res);
                }
                PluginTaskRequest::ExecuteCommand { args } => {
                    execute_command_internal(&lua, &table_key, args);
                }
                PluginTaskRequest::EmitEvent {
                    name: ev_name,
                    data,
                } => {
                    execute_event_internal(&lua, &table_key, &ev_name, &data);
                }
            }
        }
    });

    Ok(())
}

fn execute_peek_internal(
    lua: &mlua::Lua,
    table_key: &mlua::RegistryKey,
    job: PreviewJob,
) -> Option<PluginWidget> {
    let table: mlua::Table = match lua.registry_value(table_key) {
        Ok(t) => t,
        Err(_) => return None,
    };
    if let Ok(peek_fn) = table.get::<_, mlua::Function>("peek") {
        let job_table = match lua.create_table() {
            Ok(t) => t,
            Err(_) => return None,
        };
        let file_table = match lua.create_table() {
            Ok(t) => t,
            Err(_) => return None,
        };
        let _ = file_table.set("url", job.file_path.to_string_lossy().to_string());
        let _ = file_table.set("path", job.file_path.to_string_lossy().to_string());
        let _ = job_table.set("file", file_table);

        let area_table = match lua.create_table() {
            Ok(t) => t,
            Err(_) => return None,
        };
        let _ = area_table.set("width", job.area_width);
        let _ = area_table.set("height", job.area_height);
        let _ = job_table.set("area", area_table);
        let _ = job_table.set("skip", job.skip);

        // Call peek(job)
        let result: mlua::Value = match peek_fn.call((table, job_table)) {
            Ok(val) => val,
            Err(e) => {
                log::error!("Error in plugin peek: {:?}", e);
                return None;
            }
        };

        // Parse result into PluginWidget
        lua.from_value(result).ok()
    } else {
        None
    }
}

fn execute_command_internal(lua: &mlua::Lua, table_key: &mlua::RegistryKey, args: Vec<String>) {
    let table: mlua::Table = match lua.registry_value(table_key) {
        Ok(t) => t,
        Err(_) => return,
    };
    if let Ok(entry_fn) = table.get::<_, mlua::Function>("entry") {
        let args_table = match lua.create_table() {
            Ok(t) => t,
            Err(_) => return,
        };
        for (i, arg) in args.iter().enumerate() {
            let _ = args_table.set(i + 1, arg.clone());
        }
        let _: Result<(), mlua::Error> = entry_fn.call((table, args_table));
    }
}

fn execute_event_internal(
    lua: &mlua::Lua,
    _table_key: &mlua::RegistryKey,
    event_name: &str,
    data: &str,
) {
    // Look up callbacks for event in global Pub/Sub channel list
    let globals = lua.globals();
    if let Ok(pairee_table) = globals.get::<_, mlua::Table>("pairee") {
        if let Ok(ps_table) = pairee_table.get::<_, mlua::Table>("ps") {
            if let Ok(callbacks) = ps_table.get::<_, mlua::Table>("_callbacks") {
                if let Ok(callback_list) = callbacks.get::<_, mlua::Table>(event_name) {
                    let parsed_data: mlua::Value =
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                            lua.to_value(&val).unwrap_or(mlua::Value::Nil)
                        } else {
                            mlua::Value::Nil
                        };
                    let len = callback_list.len().unwrap_or(0);
                    for i in 1..=len {
                        if let Ok(func) = callback_list.get::<_, mlua::Function>(i) {
                            let _: Result<(), mlua::Error> = func.call(parsed_data.clone());
                        }
                    }
                }
            }
        }
    }
}

pub async fn run_previewer(name: &str, job: PreviewJob) -> Option<PluginWidget> {
    let registry = get_registry();
    let channels = registry.channels.read().await;
    if let Some(tx) = channels.get(name) {
        let (reply_tx, reply_rx) = oneshot::channel();
        if tx
            .send(PluginTaskRequest::Peek { job, reply_tx })
            .await
            .is_ok()
        {
            reply_rx.await.ok().flatten()
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn run_command(name: &str, args: Vec<String>) {
    let registry = get_registry();
    let plugins = registry.plugins.read().await;
    if let Some(info) = plugins.get(name) {
        log::debug!(
            "Running plugin command for {} (path: {:?})",
            name,
            info.path
        );
    }
    let channels = registry.channels.read().await;
    if let Some(tx) = channels.get(name) {
        let _ = tx.send(PluginTaskRequest::ExecuteCommand { args }).await;
    }
}

pub async fn emit_hook_event(plugin_name: &str, event_name: &str, data: String) {
    let registry = get_registry();
    let channels = registry.channels.read().await;
    if let Some(tx) = channels.get(plugin_name) {
        let _ = tx
            .send(PluginTaskRequest::EmitEvent {
                name: event_name.to_string(),
                data,
            })
            .await;
    }
}

pub async fn get_loaded_plugins() -> Vec<PluginInfo> {
    let registry = get_registry();
    registry.plugins.read().await.values().cloned().collect()
}

pub async fn resolve_keybinding(key_str: &str) -> Option<(String, String)> {
    let registry = get_registry();
    registry.keybindings.read().await.get(key_str).cloned()
}
