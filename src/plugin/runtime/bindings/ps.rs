//! `pairee.ps`: publish / subscribe between plugins. Messages carry the
//! sender recorded here in Rust (`origin`), and untrusted plugins cannot
//! publish under the host's reserved names (see `plugin::hooks`).

use crate::plugin::manager::PluginOrigin;
use tokio::sync::Semaphore;

/// Largest `pairee.ps.pub` payload, as JSON text.
const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;

/// `pairee.ps.pub` messages (of all plugins) being delivered at once.
static PENDING: Semaphore = Semaphore::const_new(64);

pub fn bind(lua: &mlua::Lua, origin: PluginOrigin) -> mlua::Result<mlua::Table> {
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
        lua.create_function(move |lua_ctx, (event_name, data): (String, mlua::Value)| {
            if !origin.trusted && crate::plugin::hooks::is_reserved(&event_name) {
                return Err(mlua::Error::runtime(format!(
                    "pairee.ps.pub: '{event_name}' is reserved for Pairee events"
                )));
            }
            let json = crate::plugin::runtime::lua_json::to_json(lua_ctx, data)?.to_string();
            if json.len() > MAX_PAYLOAD_BYTES {
                return Err(mlua::Error::runtime(format!(
                    "pairee.ps.pub: the payload is larger than {MAX_PAYLOAD_BYTES} bytes"
                )));
            }
            // Each message in flight holds a copy of the payload: bound them.
            let permit = PENDING.try_acquire().map_err(|_| {
                mlua::Error::runtime("pairee.ps.pub: too many messages are still being delivered")
            })?;
            let sender = origin.clone();
            tokio::spawn(async move {
                let _permit = permit;
                crate::plugin::hooks::publish(sender, &event_name, &json).await;
            });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_payloads_are_refused() {
        let lua = mlua::Lua::new();
        let origin = PluginOrigin {
            name: "demo".into(),
            trusted: true,
        };
        lua.globals()
            .set("ps", bind(&lua, origin).unwrap())
            .unwrap();
        let err = lua
            .load("ps.pub('big', string.rep('x', 2 * 1024 * 1024))")
            .exec()
            .unwrap_err();
        assert!(err.to_string().contains("larger than"), "{err}");
    }
}
