//! The task that runs one plugin's Lua state.
//!
//! Every request of [`PluginTaskRequest`] becomes an asynchronous call of a
//! method of the plugin table (`M:peek(job)`, `M:entry(args)`, ...) or of
//! its `pairee.ps` callbacks, so the plugin may await dialogs,
//! `pairee.sync` and child processes from any of them. Each call runs
//! under the CPU watchdog ([`guarded_async`]) and, when the method is
//! annotated `--- @sync`, after `pairee.cx` was refreshed from the main
//! loop. Requests are handled one at a time, in order.

mod preview;

pub use preview::{PreviewTarget, decode_images};

use crate::plugin::annotations::Annotations;
use crate::plugin::limits::guarded_async;
use crate::plugin::manager::{PluginOrigin, PluginRequest};
use crate::plugin::registry::PluginTaskRequest;
use crate::plugin::runtime::bindings::sync::{clear_snapshot, refresh_context};
use crate::plugin::runtime::lua_json;
use mlua::{IntoLuaMulti, Lua, MultiValue, RegistryKey, Table, Value};
use tokio::sync::mpsc;

/// One plugin's Lua state and what the host needs to call into it.
pub struct Worker {
    name: String,
    lua: Lua,
    table_key: RegistryKey,
    annotations: Annotations,
    tx: mpsc::Sender<PluginRequest>,
    preloaded: preview::PreloadCache,
}

impl Worker {
    pub fn new(
        name: String,
        lua: Lua,
        table_key: RegistryKey,
        annotations: Annotations,
        tx: mpsc::Sender<PluginRequest>,
    ) -> Self {
        Self {
            name,
            lua,
            table_key,
            annotations,
            tx,
            preloaded: preview::PreloadCache::default(),
        }
    }

    async fn handle(&mut self, req: PluginTaskRequest) {
        match req {
            PluginTaskRequest::Peek { job, reply_tx } => {
                let _ = reply_tx.send(self.peek(job).await);
            }
            PluginTaskRequest::Preload { job, reply_tx } => {
                let _ = reply_tx.send(self.preload(job).await);
            }
            PluginTaskRequest::Seek {
                job,
                units,
                reply_tx,
            } => {
                let _ = reply_tx.send(self.seek(job, units).await);
            }
            PluginTaskRequest::ExecuteCommand { args } => self.entry(args).await,
            PluginTaskRequest::EmitEvent { name, data, sender } => {
                self.event(&name, &data, sender.as_ref()).await
            }
        }
    }

    /// Calls `M:<method>(arg)`; `Ok(None)` when the plugin has no such
    /// method. Errors are the plugin's (or the watchdog's).
    async fn call_method(
        &self,
        method: &str,
        arg: impl IntoLuaMulti,
    ) -> mlua::Result<Option<MultiValue>> {
        let table: Table = self.lua.registry_value(&self.table_key)?;
        let Value::Function(func) = table.get::<Value>(method)? else {
            return Ok(None);
        };
        let sync = self.annotations.is_sync(method);
        if sync {
            refresh_context(&self.lua, &self.tx).await?;
        }
        let args = (table, arg).into_lua_multi(&self.lua)?;
        let result = guarded_async(&self.lua, func.call_async::<MultiValue>(args)).await;
        if sync {
            clear_snapshot(&self.lua)?;
        }
        result.map(Some)
    }

    /// Logs a failed call of `what`.
    fn report<T>(&self, what: &str, result: mlua::Result<T>) -> Option<T> {
        result
            .inspect_err(|e| tracing::error!("Plugin {} failed in {what}: {e}", self.name))
            .ok()
    }

    pub(crate) async fn entry(&self, args: Vec<String>) {
        let result = async {
            let table = self.lua.create_sequence_from(args.iter().cloned())?;
            if let Some(command) = args.first() {
                table.set("command", command.as_str())?;
            }
            self.call_method("entry", table).await
        }
        .await;
        self.report("entry", result);
    }

    /// Runs the plugin's `pairee.ps.sub` callbacks for `event` with the
    /// payload and who sent it (`sender`, recorded by Rust).
    pub(crate) async fn event(&self, event: &str, data: &str, sender: Option<&PluginOrigin>) {
        let callbacks = self.lua.globals().get::<Table>("pairee").and_then(|p| {
            p.get::<Table>("ps")?
                .get::<Table>("_callbacks")?
                .get::<Option<Table>>(event)
        });
        let Some(callbacks) = self.report(event, callbacks).flatten() else {
            return;
        };
        let payload: Value = serde_json::from_str::<serde_json::Value>(data)
            .ok()
            .and_then(|val| lua_json::from_json(&self.lua, &val).ok())
            .unwrap_or(Value::Nil);
        let Some(meta) = self.report(event, event_meta(&self.lua, sender)) else {
            return;
        };
        let funcs: Vec<mlua::Function> = callbacks.sequence_values().flatten().collect();
        for func in funcs {
            let call = func.call_async::<()>((payload.clone(), meta.clone()));
            self.report(event, guarded_async(&self.lua, call).await);
        }
    }
}

/// The second argument of a `pairee.ps.sub` callback: `from_host` for host
/// hooks; `sender` (plugin name) and `trusted` for `pairee.ps.pub`
/// messages. Built from what Rust recorded, so a plugin cannot fake it.
fn event_meta(lua: &mlua::Lua, sender: Option<&PluginOrigin>) -> mlua::Result<Table> {
    let meta = lua.create_table()?;
    meta.set("from_host", sender.is_none())?;
    if let Some(origin) = sender {
        meta.set("sender", origin.name.as_str())?;
        meta.set("trusted", origin.trusted)?;
    }
    Ok(meta)
}

/// Starts the worker task; it ends when the registry drops the channel.
pub fn spawn(mut worker: Worker, mut rx: mpsc::Receiver<PluginTaskRequest>) {
    tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            worker.handle(req).await;
        }
    });
}

#[cfg(test)]
mod tests;
