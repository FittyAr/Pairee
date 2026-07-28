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
    /// Whether the plugin was loaded with `trusted = true` (the
    /// trust flag grants access to the full Lua standard library
    /// and to process spawning). Used by the hook broadcaster
    /// (e.g. `on_key`) to keep sensitive key events away from
    /// untrusted plugins in Secure Mode.
    pub trusted: bool,
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
    trusted: bool,
    resolver: &std::sync::Arc<crate::keybindings::resolver::KeybindingResolver>,
) -> anyhow::Result<()> {
    let name = manifest.name.clone();
    let registry = get_registry();

    // Insert into plugins list
    let info = PluginInfo {
        manifest: manifest.clone(),
        path: path.clone(),
        trusted,
    };
    registry.plugins.write().await.insert(name.clone(), info);

    // Register keybindings. Two things happen here:
    //   1. The plugin registry's `keybindings` map (legacy
    //      storage, kept for the CLI command `pairee plugin
    //      list-keys` and the upcoming Conflicts sub-screen) is
    //      updated.
    //   2. The live `KeybindingResolver` is asked to install each
    //      binding. The resolver applies the priority + conflict
    //      rules from `keybindings::source` and logs (and
    //      optionally toasts) any conflicts it finds.
    if let Some(ref keymaps) = manifest.keybindings {
        let mut keybindings = registry.keybindings.write().await;
        for (key, action) in keymaps {
            keybindings.insert(key.clone(), (name.clone(), action.clone()));
            // Mirror the binding into the resolver. Manifest
            // entries use the default `Fallback` policy so a
            // plugin never silently hijacks a Builtin key.
            let outcome = resolver.register(
                key,
                crate::keybindings::source::ResolvedBinding::plugin(&name, action),
                crate::keybindings::source::ConflictPolicy::Fallback,
            );
            match outcome {
                crate::keybindings::source::RegisterOutcome::Bound => {
                    log::info!(
                        "plugin '{}': registered keybinding '{}' -> '{}'",
                        name,
                        key,
                        action
                    );
                }
                crate::keybindings::source::RegisterOutcome::Conflict { with } => {
                    log::warn!(
                        "plugin '{}': keybinding '{}' -> '{}' refused (key already owned by {}). \
                         Adjust the manifest's [keybindings] entry, unbind the key in your \
                         keybindings.toml, or use ConflictPolicy = 'override' (future API).",
                        name,
                        key,
                        action,
                        with
                    );
                }
                crate::keybindings::source::RegisterOutcome::Invalid => {
                    log::warn!(
                        "plugin '{}': keybinding '{}' is invalid (empty or malformed)",
                        name,
                        key
                    );
                }
            }
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

        // Parse result into PluginWidget. Plugins can return
        // either:
        //   - a `Renderable` userdata (new M4 path — built via
        //     `ui.Span(...)`, `ui.Line(...)`, etc., with builder
        //     chain), or
        //   - a plain Lua table with the `type` discriminator
        //     (legacy serde path).
        peek_value_to_plugin(&lua, result)
    } else {
        None
    }
}

/// Convert a peek() return value into a `PluginWidget`. Handles
/// both the new userdata-backed `Renderable` enum (built via
/// `ui.Span/Line/Text/Paragraph/List/Gauge/Table`) and the legacy
/// plain-table form (serde-deserialized via the `type` field).
fn peek_value_to_plugin(lua: &mlua::Lua, val: mlua::Value) -> Option<PluginWidget> {
    use crate::plugin::runtime::bindings::ui::preview::widget_to_plugin;
    // First try the userdata-backed widgets (new M4 path).
    if let mlua::Value::UserData(_) = &val {
        if let Ok(pw) = widget_to_plugin(val.clone()) {
            // §N1/N2: cap depth + truncate oversized strings
            // before the widget leaves the plugin worker.
            let mut sanitized = pw;
            crate::plugin::runtime::bindings::ui::preview::sanitize_plugin_widget(&mut sanitized);
            return Some(sanitized);
        }
    }
    // Fall back to the legacy serde-deserialized form.
    let mut legacy = lua.from_value(val).ok()?;
    crate::plugin::runtime::bindings::ui::preview::sanitize_plugin_widget(&mut legacy);
    Some(legacy)
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

/// Shared `peek` job-table builder.
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
