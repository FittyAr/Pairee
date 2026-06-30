use crate::plugin::manager::PluginRequest;
use mlua::prelude::*;
use tokio::sync::mpsc;

pub fn bind(lua: &mlua::Lua, _tx: mpsc::Sender<PluginRequest>) -> mlua::Result<mlua::Table<'_>> {
    let ps = lua.create_table()?;
    let callbacks = lua.create_table()?;
    ps.set("_callbacks", callbacks)?;

    ps.set(
        "sub",
        lua.create_function(|lua_ctx, (event_name, func): (String, mlua::Function)| {
            let globals = lua_ctx.globals();
            let pairee: mlua::Table = globals.get("pairee")?;
            let ps_table: mlua::Table = pairee.get("ps")?;
            let callbacks_table: mlua::Table = ps_table.get("_callbacks")?;

            let list: mlua::Table = match callbacks_table.get(event_name.as_str()) {
                Ok(l) => l,
                Err(_) => {
                    let l = lua_ctx.create_table()?;
                    callbacks_table.set(event_name.clone(), l.clone())?;
                    l
                }
            };

            let len = list.len().unwrap_or(0);
            list.set(len + 1, func)?;
            Ok(())
        })?,
    )?;

    ps.set(
        "pub",
        lua.create_function(|lua_ctx, (event_name, data): (String, mlua::Value)| {
            if let Ok(serialized) = lua_ctx.to_value(&data) {
                if let Ok(json_val) = serde_json::to_value(&serialized) {
                    tokio::spawn(async move {
                        crate::plugin::hooks::emit_event(&event_name, json_val).await;
                    });
                }
            }
            Ok(())
        })?,
    )?;

    ps.set(
        "unsub",
        lua.create_function(|lua_ctx, event_name: String| {
            let globals = lua_ctx.globals();
            let pairee: mlua::Table = globals.get("pairee")?;
            let ps_table: mlua::Table = pairee.get("ps")?;
            let callbacks_table: mlua::Table = ps_table.get("_callbacks")?;
            let _: Result<(), mlua::Error> = callbacks_table.set(event_name, mlua::Value::Nil);
            Ok(())
        })?,
    )?;

    Ok(ps)
}
