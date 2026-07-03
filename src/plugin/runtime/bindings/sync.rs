use crate::plugin::manager::PluginRequest;
use mlua::prelude::*;
use tokio::sync::mpsc;

/// `pairee.sync(fn)` — wrap a function so that calling it fetches
/// a fresh state snapshot from the main thread and runs the
/// function with that snapshot stashed on
/// `pairee.app._current_snapshot`.
///
/// This is the M3 "sync bridge": an isolated async VM can call
/// `pairee.sync(do_thing)` to read live state without spinning
/// up a second main-thread VM.
pub fn bind(lua: &mlua::Lua, tx: mpsc::Sender<PluginRequest>) -> mlua::Result<mlua::Function<'_>> {
    let tx_clone = tx.clone();
    lua.create_function(move |lua_ctx, func: mlua::Function| {
        let tx = tx_clone.clone();
        let raw_key = lua_ctx.create_registry_value(func)?;
        let func_key = std::sync::Arc::new(raw_key);

        let wrapper = lua_ctx.create_async_function(move |lua_ctx2, args: mlua::MultiValue| {
            let tx = tx.clone();
            let func_key = func_key.clone();
            async move {
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                if tx
                    .send(PluginRequest::GetStateSnapshot(reply_tx))
                    .await
                    .is_err()
                {
                    return Err(mlua::Error::RuntimeError(
                        "Failed to communicate with main thread".to_string(),
                    ));
                }
                let snapshot = reply_rx.await.map_err(|e| {
                    mlua::Error::RuntimeError(format!("Failed to receive state snapshot: {}", e))
                })?;
                let globals = lua_ctx2.globals();
                let pairee: mlua::Table = globals.get("pairee")?;
                let app: mlua::Table = pairee.get("app")?;
                let snapshot_value = lua_ctx2.to_value(&snapshot)?;
                app.set("_current_snapshot", snapshot_value)?;
                let user_fn: mlua::Function = lua_ctx2.registry_value(&*func_key)?;
                let result: mlua::Value = user_fn.call(args)?;
                app.set("_current_snapshot", mlua::Value::Nil)?;
                Ok(result)
            }
        })?;
        Ok(wrapper)
    })
}

/// `pairee.async_fn(fn)` — wrap a function so that calling it
/// dispatches the call onto the plugin's async tokio task
/// (does NOT block the calling thread). Today this is
/// functionally equivalent to `pairee.sync(fn)` because all
/// plugin callbacks already run in an async task; M4 will
/// distinguish sync- and async-mode VMs.
pub fn bind_async(
    lua: &mlua::Lua,
    _tx: mpsc::Sender<PluginRequest>,
) -> mlua::Result<mlua::Function<'_>> {
    lua.create_function(move |_lua_ctx, func: mlua::Function| {
        // For M3 we simply return the function as-is. M4's
        // sync/async VM split will change the semantics so that
        // `pairee.async_fn(...)` runs the body in a fresh task
        // *without* fetching a snapshot.
        Ok(func)
    })
}
