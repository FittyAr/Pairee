use crate::plugin::manager::PluginRequest;
use mlua::prelude::*;
use tokio::sync::mpsc;

pub fn bind(lua: &mlua::Lua, tx: mpsc::Sender<PluginRequest>) -> mlua::Result<mlua::Function<'_>> {
    let tx_clone = tx.clone();

    lua.create_function(move |lua_ctx, func: mlua::Function| {
        let tx = tx_clone.clone();

        let raw_key = lua_ctx.create_registry_value(func)?;
        let func_key = std::sync::Arc::new(raw_key);

        let wrapper = lua_ctx.create_async_function(move |lua_ctx2, ()| {
            let tx = tx.clone();
            let func_key = func_key.clone();

            async move {
                // 1. Send snapshot request to main thread
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

                // 2. Await snapshot
                let snapshot = reply_rx.await.map_err(|e| {
                    mlua::Error::RuntimeError(format!("Failed to receive state snapshot: {}", e))
                })?;

                // 3. Set snapshot in pairee.app._current_snapshot
                let globals = lua_ctx2.globals();
                let pairee: mlua::Table = globals.get("pairee")?;
                let app: mlua::Table = pairee.get("app")?;

                let snapshot_value = lua_ctx2.to_value(&snapshot)?;
                app.set("_current_snapshot", snapshot_value)?;

                // 4. Run the user function
                let user_fn: mlua::Function = lua_ctx2.registry_value(&*func_key)?;
                let result: mlua::Value = user_fn.call(())?;

                // 5. Clear snapshot
                app.set("_current_snapshot", mlua::Value::Nil)?;

                Ok(result)
            }
        })?;

        Ok(wrapper)
    })
}
