//! Live-ish application context (`pairee.cx`) built from the latest snapshot.

use crate::plugin::manager::snapshot::AppStateSnapshot;
use crate::plugin::runtime::types::LuaFile;
use mlua::{Lua, Table, Value};

/// Empty `pairee.cx` so plugins can read the table before the first sync.
pub fn bind_empty(lua: &Lua, pairee: &Table<'_>) -> mlua::Result<()> {
    let cx = lua.create_table()?;
    let active = lua.create_table()?;
    active.set("cwd", "")?;
    active.set("hovered", Value::Nil)?;
    active.set("selected", lua.create_table()?)?;
    let current = lua.create_table()?;
    current.set("cwd", "")?;
    current.set("hovered", Value::Nil)?;
    active.set("current", current)?;
    cx.set("active", active)?;

    let left = lua.create_table()?;
    left.set("cwd", "")?;
    cx.set("left", left)?;

    let right = lua.create_table()?;
    right.set("cwd", "")?;
    cx.set("right", right)?;

    pairee.set("cx", cx)?;
    Ok(())
}

/// Refresh `pairee.cx` from a main-thread snapshot (called inside `pairee.sync`).
pub fn install(lua: &Lua, snapshot: &AppStateSnapshot) -> mlua::Result<()> {
    let globals = lua.globals();
    let pairee: Table = globals.get("pairee")?;
    let cx = lua.create_table()?;

    let active_cwd = if snapshot.active_panel == "right" {
        snapshot.right_cwd.as_str()
    } else {
        snapshot.left_cwd.as_str()
    };

    let hovered = match &snapshot.hovered_file {
        Some(entry) => Value::UserData(lua.create_userdata(LuaFile::from_snapshot(entry))?),
        None => Value::Nil,
    };

    let selected = lua.create_table()?;
    for (i, entry) in snapshot.selected_files.iter().enumerate() {
        selected.set(i + 1, LuaFile::from_snapshot(entry))?;
    }

    let current = lua.create_table()?;
    current.set("cwd", active_cwd)?;
    current.set("hovered", hovered.clone())?;

    let active = lua.create_table()?;
    active.set("cwd", active_cwd)?;
    active.set("hovered", hovered)?;
    active.set("selected", selected)?;
    active.set("current", current)?;
    cx.set("active", active)?;

    let left = lua.create_table()?;
    left.set("cwd", snapshot.left_cwd.as_str())?;
    cx.set("left", left)?;

    let right = lua.create_table()?;
    right.set("cwd", snapshot.right_cwd.as_str())?;
    cx.set("right", right)?;

    pairee.set("cx", cx)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::manager::snapshot::FileEntrySnapshot;
    use mlua::Lua;

    fn sample_snapshot() -> AppStateSnapshot {
        AppStateSnapshot {
            active_panel: "left".into(),
            left_cwd: "/left".into(),
            right_cwd: "/right".into(),
            hovered_file: Some(FileEntrySnapshot {
                name: "a.txt".into(),
                url: "/left/a.txt".into(),
                path: "/left/a.txt".into(),
                size: 4,
                is_dir: false,
                is_symlink: false,
            }),
            selected_files: vec![FileEntrySnapshot {
                name: "b.txt".into(),
                url: "/left/b.txt".into(),
                path: "/left/b.txt".into(),
                size: 2,
                is_dir: false,
                is_symlink: false,
            }],
        }
    }

    #[test]
    fn install_exposes_cwd_and_file_userdata() {
        let lua = Lua::new();
        let pairee = lua.create_table().unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        install(&lua, &sample_snapshot()).unwrap();

        let cwd: String = lua.load("return pairee.cx.active.cwd").eval().unwrap();
        let name: String = lua
            .load("return pairee.cx.active.hovered.name")
            .eval()
            .unwrap();
        let selected_n: i64 = lua
            .load("return #pairee.cx.active.selected")
            .eval()
            .unwrap();
        let nested: String = lua
            .load("return pairee.cx.active.current.hovered.path")
            .eval()
            .unwrap();
        let right: String = lua.load("return pairee.cx.right.cwd").eval().unwrap();

        assert_eq!(cwd, "/left");
        assert_eq!(name, "a.txt");
        assert_eq!(selected_n, 1);
        assert_eq!(nested, "/left/a.txt");
        assert_eq!(right, "/right");
    }

    #[test]
    fn bind_empty_has_safe_defaults() {
        let lua = Lua::new();
        let pairee = lua.create_table().unwrap();
        bind_empty(&lua, &pairee).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        let cwd: String = lua.load("return pairee.cx.active.cwd").eval().unwrap();
        let hovered: mlua::Value = lua.load("return pairee.cx.active.hovered").eval().unwrap();
        assert_eq!(cwd, "");
        assert!(hovered.is_nil());
    }
}
