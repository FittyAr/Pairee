use crate::app::state::types::PluginWidget;
use crate::plugin::loader::PluginManifest;
use crate::plugin::types::File;
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
    /// M3: call the plugin's `preload(job)` function. The reply
    /// tuple is `(complete: bool, err: Option<String>)`: the
    /// previewer signals whether it is done with this file and
    /// can be evicted from the cache.
    Preload {
        job: PreviewJob,
        reply_tx: oneshot::Sender<(bool, Option<String>)>,
    },
    /// M3: call the plugin's `seek(job)` function with a new
    /// `skip` value. The previewer returns the resulting widget
    /// so the UI can replace the rendered preview in place.
    Seek {
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
                PluginTaskRequest::Preload { job, reply_tx } => {
                    let res = execute_preload_internal(&lua, &table_key, job);
                    let _ = reply_tx.send(res);
                }
                PluginTaskRequest::Seek { job, reply_tx } => {
                    let res = execute_seek_internal(&lua, &table_key, job);
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
        let job_table = build_job_table(lua, &job);
        // M2-T6: attach a real `File` userdata so plugins can
        // call `job.file.cha:perm()`, `job.file:size()`, etc. We
        // also keep the legacy `file.url`/`file.path` string fields
        // for older plugins.
        if let Some(file_table) = job_table.get::<_, mlua::Table>("file").ok() {
            let url = crate::plugin::types::Url::parse(&job.file_path.to_string_lossy());
            let file_ud = match std::fs::metadata(&job.file_path) {
                Ok(meta) => {
                    let f = File::from_url_and_metadata(url.clone(), meta, true);
                    lua.create_userdata(f).ok()
                }
                Err(_) => {
                    let f = File::from_url(url.clone());
                    lua.create_userdata(f).ok()
                }
            };
            if let Some(ud) = file_ud {
                let _ = file_table.set("userdata", ud);
            }
        }

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

/// Calls the plugin's `preload(job)` Lua function and returns
/// `(complete, err)`. The reply is `true` on success; an error
/// is `Some("...")` if the plugin threw.
fn execute_preload_internal(
    lua: &mlua::Lua,
    table_key: &mlua::RegistryKey,
    job: PreviewJob,
) -> (bool, Option<String>) {
    let table: mlua::Table = match lua.registry_value(table_key) {
        Ok(t) => t,
        Err(_) => return (true, Some("plugin table missing".to_string())),
    };
    let preload_fn = match table.get::<_, mlua::Function>("preload") {
        Ok(f) => f,
        Err(_) => return (true, None), // no preload → cacheable by default
    };
    let job_table = build_job_table(lua, &job);
    match preload_fn.call::<_, ()>((table, job_table)) {
        Ok(()) => (true, None),
        Err(e) => (false, Some(format!("{e}"))),
    }
}

/// Calls the plugin's `seek(job)` Lua function. Returns the
/// resulting widget (or `None` on error / if the plugin does
/// not implement `seek`).
fn execute_seek_internal(
    lua: &mlua::Lua,
    table_key: &mlua::RegistryKey,
    job: PreviewJob,
) -> Option<PluginWidget> {
    let table: mlua::Table = lua.registry_value(table_key).ok()?;
    let seek_fn = table.get::<_, mlua::Function>("seek").ok()?;
    let job_table = build_job_table(lua, &job);
    let result: mlua::Value = match seek_fn.call((table, job_table)) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Error in plugin seek: {:?}", e);
            return None;
        }
    };
    lua.from_value(result).ok()
}

/// Shared `peek`/`preload`/`seek` job-table builder.
fn build_job_table<'lua>(lua: &'lua mlua::Lua, job: &PreviewJob) -> mlua::Table<'lua> {
    let job_table = lua.create_table().unwrap_or_else(|_| {
        // create_table doesn't fail in practice for an empty
        // table; fall back to an empty table by reaching through
        // globals (which always exist).
        lua.globals()
    });
    let file_table = lua.create_table().unwrap_or_else(|_| lua.globals());
    let _ = file_table.set("url", job.file_path.to_string_lossy().to_string());
    let _ = file_table.set("path", job.file_path.to_string_lossy().to_string());
    let _ = job_table.set("file", file_table);
    let area_table = lua.create_table().unwrap_or_else(|_| lua.globals());
    let _ = area_table.set("width", job.area_width);
    let _ = area_table.set("height", job.area_height);
    let _ = job_table.set("area", area_table);
    let _ = job_table.set("skip", job.skip);
    job_table
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

/// M3: ask the plugin to preload a file. Returns `true` when the
/// previewer has finished with the file (the cache can evict it)
/// and `false` on error.
pub async fn run_preloader(name: &str, job: PreviewJob) -> (bool, Option<String>) {
    let registry = get_registry();
    let channels = registry.channels.read().await;
    if let Some(tx) = channels.get(name) {
        let (reply_tx, reply_rx) = oneshot::channel();
        if tx
            .send(PluginTaskRequest::Preload { job, reply_tx })
            .await
            .is_ok()
        {
            reply_rx.await.unwrap_or((true, None))
        } else {
            (true, Some("channel closed".to_string()))
        }
    } else {
        (true, Some("plugin not loaded".to_string()))
    }
}

/// M3: ask the plugin to seek inside a file. Returns the new
/// rendered widget.
pub async fn run_seeker(name: &str, job: PreviewJob) -> Option<PluginWidget> {
    let registry = get_registry();
    let channels = registry.channels.read().await;
    if let Some(tx) = channels.get(name) {
        let (reply_tx, reply_rx) = oneshot::channel();
        if tx
            .send(PluginTaskRequest::Seek { job, reply_tx })
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
