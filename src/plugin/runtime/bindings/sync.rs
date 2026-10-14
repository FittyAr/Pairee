//! `pairee.sync(fn)`: runs `fn` with `pairee.cx` (and the legacy
//! `pairee.app` snapshot) refreshed from the main loop. The same refresh
//! runs before methods annotated `--- @sync` (see `plugin::annotations`).

use crate::plugin::manager::PluginRequest;
use mlua::prelude::*;
use tokio::sync::mpsc;

/// Asks the main loop for a state snapshot and installs it as
/// `pairee.cx` and `pairee.app._current_snapshot`.
pub async fn refresh_context(lua: &Lua, tx: &mpsc::Sender<PluginRequest>) -> LuaResult<()> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(PluginRequest::GetStateSnapshot(reply_tx))
        .await
        .map_err(|_| LuaError::RuntimeError("Failed to communicate with main thread".into()))?;
    let snapshot = reply_rx
        .await
        .map_err(|e| LuaError::RuntimeError(format!("Failed to receive state snapshot: {e}")))?;
    let app = app_table(lua)?;
    app.set("_current_snapshot", lua.to_value(&snapshot)?)?;
    super::cx::install_snapshot(lua, &snapshot)
}

/// Drops the legacy snapshot again (`pairee.cx` stays as last seen).
pub fn clear_snapshot(lua: &Lua) -> LuaResult<()> {
    app_table(lua)?.set("_current_snapshot", LuaValue::Nil)
}

fn app_table(lua: &Lua) -> LuaResult<LuaTable> {
    lua.globals().get::<LuaTable>("pairee")?.get("app")
}

pub fn bind(lua: &Lua, tx: mpsc::Sender<PluginRequest>) -> LuaResult<LuaFunction> {
    lua.create_function(move |lua_ctx, func: LuaFunction| {
        let tx = tx.clone();
        let func_key = std::sync::Arc::new(lua_ctx.create_registry_value(func)?);
        lua_ctx.create_async_function(move |lua, ()| {
            let tx = tx.clone();
            let func_key = func_key.clone();
            async move {
                refresh_context(&lua, &tx).await?;
                let user_fn: LuaFunction = lua.registry_value(&func_key)?;
                let result: LuaValue = user_fn.call_async(()).await?;
                clear_snapshot(&lua)?;
                Ok(result)
            }
        })
    })
}
