//! `pairee.state` — per-plugin mutable state.
//!
//! Plugins get a Lua table that persists across calls (counter,
//! cache, last-seen timestamp, …). The table is stored in a
//! `mlua::RegistryKey` so it lives as long as the plugin's
//! `Lua` instance; the runtime holds a `Arc<RegistryKey>` so it
//! can hand the same table to other callbacks if/when needed.
//!
//! M3 implementation: each plugin's `main.lua` evaluates to a
//! table; that table is registered under the plugin name and
//! `pairee.state` returns the same table so the plugin can
//! mutate it freely across callbacks.

use mlua::RegistryKey;

/// Bind the per-plugin `pairee.state` to the given table and
/// store the `RegistryKey` on the `Runtime` so other bindings
/// (e.g. `pairee.sync(fn)`) can find the same state when they
/// are called from the plugin's worker.
pub fn attach(
    lua: &mlua::Lua,
    runtime: &crate::plugin::runtime::runtime::Runtime,
    plugin_name: &str,
    state_table: &mlua::Table<'_>,
) -> mlua::Result<()> {
    let key: RegistryKey = lua.create_registry_value(state_table.clone())?;
    runtime.register_plugin_state(plugin_name, key);
    // Also register the table on `pairee.state` for the plugin.
    let globals = lua.globals();
    if let Ok(pairee) = globals.get::<_, mlua::Table>("pairee") {
        pairee.set("state", state_table.clone())?;
    }
    Ok(())
}

/// Re-attach an already-stored state to a fresh `pairee` table
/// (e.g. when the plugin's `Lua` instance is recreated for a
/// fresh sync). Returns the table or `None` if the plugin has
/// no state.
pub fn reattach<'lua>(
    lua: &'lua mlua::Lua,
    runtime: &crate::plugin::runtime::runtime::Runtime,
    plugin_name: &str,
) -> mlua::Result<Option<mlua::Table<'lua>>> {
    if let Some(key) = runtime.plugin_state_key(plugin_name) {
        let table: mlua::Table = lua.registry_value(&key)?;
        let globals = lua.globals();
        if let Ok(pairee) = globals.get::<_, mlua::Table>("pairee") {
            pairee.set("state", table.clone())?;
        }
        Ok(Some(table))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::runtime::runtime::Runtime;
    use mlua::Lua;

    #[test]
    fn test_attach_publishes_pairee_state_global() {
        let lua = Lua::new();
        let runtime = Runtime::new();
        let pair = lua.create_table().unwrap();
        lua.globals().set("pairee", pair).unwrap();
        let table = lua.create_table().unwrap();
        table.set("count", 42i64).unwrap();
        attach(&lua, &runtime, "test_plugin", &table).unwrap();
        // After attach, the state is reachable via `pairee.state`.
        let pair = lua.globals().get::<_, mlua::Table>("pairee").unwrap();
        let state = pair.get::<_, mlua::Table>("state").unwrap();
        let n: i64 = state.get("count").unwrap();
        assert_eq!(n, 42);
    }

    #[test]
    fn test_reattach_unknown_plugin_returns_none() {
        // The Runtime's `plugin_state_key` is a placeholder today
        // (the cross-Lua reattach is reserved for M3.5); reattach
        // therefore returns None even for known plugins. We
        // exercise the path here to make sure the function does
        // not panic on a fresh Lua.
        let lua = Lua::new();
        let pair = lua.create_table().unwrap();
        lua.globals().set("pairee", pair).unwrap();
        let runtime = Runtime::new();
        let result = reattach(&lua, &runtime, "no_such_plugin").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_attach_overwrites_previous_state_for_same_plugin() {
        let lua = Lua::new();
        let pair = lua.create_table().unwrap();
        lua.globals().set("pairee", pair).unwrap();
        let runtime = Runtime::new();
        let t1 = lua.create_table().unwrap();
        t1.set("v", 1i64).unwrap();
        attach(&lua, &runtime, "p", &t1).unwrap();
        let t2 = lua.create_table().unwrap();
        t2.set("v", 2i64).unwrap();
        attach(&lua, &runtime, "p", &t2).unwrap();
        // The latest attach wins — `pairee.state.v == 2`.
        let pair = lua.globals().get::<_, mlua::Table>("pairee").unwrap();
        let state = pair.get::<_, mlua::Table>("state").unwrap();
        let n: i64 = state.get("v").unwrap();
        assert_eq!(n, 2);
    }
}
