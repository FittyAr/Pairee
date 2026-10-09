//! `pairee.keymap`: read-only view of the live keymap.
//!
//! - `pairee.keymap.list()` → `{ { chord, id, label, origin }, ... }`
//! - `pairee.keymap.chord_for(id)` → the first chord bound to `id`
//!   (`"copy"`, `"plugin.<name>.<command>"`), or `nil`.

use crate::keybindings::published;

pub fn bind(lua: &mlua::Lua) -> mlua::Result<mlua::Table<'_>> {
    let table = lua.create_table()?;
    table.set(
        "list",
        lua.create_function(|lua, ()| {
            let list = lua.create_table()?;
            for (i, b) in published::bindings().into_iter().enumerate() {
                let row = lua.create_table()?;
                row.set("chord", b.chord)?;
                row.set("id", b.id)?;
                row.set("label", b.label)?;
                row.set("origin", b.origin)?;
                list.set(i + 1, row)?;
            }
            Ok(list)
        })?,
    )?;
    table.set(
        "chord_for",
        lua.create_function(|_, id: String| Ok(published::chord_for(&id)))?,
    )?;
    Ok(table)
}

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;
    use crate::keybindings::KeybindingResolver;

    #[test]
    fn lua_sees_the_live_keymap() {
        let _resolver = KeybindingResolver::new(&AppConfig::default());
        let lua = mlua::Lua::new();
        lua.globals()
            .set("keymap", super::bind(&lua).unwrap())
            .unwrap();
        let chord: Option<String> = lua.load("return keymap.chord_for('copy')").eval().unwrap();
        assert_eq!(chord.as_deref(), Some("F5"));
        let count: usize = lua.load("return #keymap.list()").eval().unwrap();
        assert!(count > 100);
        let missing: Option<String> = lua.load("return keymap.chord_for('nope')").eval().unwrap();
        assert_eq!(missing, None);
    }
}
