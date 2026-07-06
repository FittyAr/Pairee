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
