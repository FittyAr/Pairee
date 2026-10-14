//! Live-ish application context (`pairee.cx`) built from the latest snapshot.
//!
//! * `cx.active` / `cx.left` / `cx.right`: `cwd` (string) and `url` (`Url`,
//!   remote for a remote panel); `cx.active` also has `hovered`,
//!   `selected` and `current` (`cwd`, `url`, `hovered`).
//! * `cx.tabs`: the tabs of the active side, each `{url, name, locked}`,
//!   plus `idx` (1-based index of the shown tab).
//! * `cx.tasks`: transfer jobs `{id, kind, status, sources, destination,
//!   files_total, files_done, bytes_total, bytes_done}`.
//! * `cx.yanked`: the file clipboard as a list of `Url`s, plus `is_cut`.

use crate::plugin::manager::snapshot::{AppStateSnapshot, TaskSnapshot};
use crate::plugin::runtime::types::{LuaFile, LuaUrl};
use mlua::{Lua, Table, Value};

/// Empty `pairee.cx` so plugins can read the table before the first sync.
pub fn bind_empty(lua: &Lua, pairee: &Table) -> mlua::Result<()> {
    install(lua, pairee, &AppStateSnapshot::default())
}

/// Refresh `pairee.cx` from a main-thread snapshot (called inside `pairee.sync`).
pub fn install_snapshot(lua: &Lua, snapshot: &AppStateSnapshot) -> mlua::Result<()> {
    let pairee: Table = lua.globals().get("pairee")?;
    install(lua, &pairee, snapshot)
}

fn install(lua: &Lua, pairee: &Table, snapshot: &AppStateSnapshot) -> mlua::Result<()> {
    let cx = lua.create_table()?;
    let right_active = snapshot.active_panel == "right";
    let (active_cwd, active_url) = if right_active {
        (&snapshot.right_cwd, &snapshot.right_url)
    } else {
        (&snapshot.left_cwd, &snapshot.left_url)
    };

    let hovered = match &snapshot.hovered_file {
        Some(entry) => Value::UserData(lua.create_userdata(LuaFile::from_snapshot(entry))?),
        None => Value::Nil,
    };
    let selected = lua.create_table()?;
    for (i, entry) in snapshot.selected_files.iter().enumerate() {
        selected.set(i + 1, LuaFile::from_snapshot(entry))?;
    }

    let current = folder(lua, active_cwd, active_url)?;
    current.set("hovered", hovered.clone())?;
    let active = folder(lua, active_cwd, active_url)?;
    active.set("hovered", hovered)?;
    active.set("selected", selected)?;
    active.set("current", current)?;
    cx.set("active", active)?;
    cx.set("left", folder(lua, &snapshot.left_cwd, &snapshot.left_url)?)?;
    cx.set(
        "right",
        folder(lua, &snapshot.right_cwd, &snapshot.right_url)?,
    )?;
    cx.set("tabs", tabs(lua, snapshot)?)?;
    cx.set("tasks", tasks(lua, &snapshot.tasks)?)?;
    cx.set("yanked", yanked(lua, snapshot)?)?;
    pairee.set("cx", cx)
}

/// `{cwd, url}` of a panel; `url` falls back to the path.
fn folder(lua: &Lua, cwd: &str, url: &str) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("cwd", cwd)?;
    let url = if url.is_empty() { cwd } else { url };
    table.set("url", LuaUrl::parse(url))?;
    Ok(table)
}

fn tabs(lua: &Lua, snapshot: &AppStateSnapshot) -> mlua::Result<Table> {
    let list = lua.create_table()?;
    for (i, tab) in snapshot.tabs.iter().enumerate() {
        let entry = lua.create_table()?;
        entry.set("url", LuaUrl::parse(&tab.url))?;
        entry.set("name", tab.name.clone())?;
        entry.set("locked", tab.locked)?;
        list.set(i + 1, entry)?;
    }
    list.set("idx", snapshot.active_tab + 1)?;
    Ok(list)
}

fn tasks(lua: &Lua, tasks: &[TaskSnapshot]) -> mlua::Result<Table> {
    let list = lua.create_table()?;
    for (i, task) in tasks.iter().enumerate() {
        let entry = lua.create_table()?;
        entry.set("id", task.id.as_str())?;
        entry.set("kind", task.kind.as_str())?;
        entry.set("status", task.status.as_str())?;
        entry.set("sources", task.sources)?;
        entry.set("destination", task.destination.as_str())?;
        entry.set("files_total", task.files_total)?;
        entry.set("files_done", task.files_done)?;
        entry.set("bytes_total", task.bytes_total)?;
        entry.set("bytes_done", task.bytes_done)?;
        list.set(i + 1, entry)?;
    }
    Ok(list)
}

fn yanked(lua: &Lua, snapshot: &AppStateSnapshot) -> mlua::Result<Table> {
    let list = lua.create_table()?;
    let clip = snapshot.yanked.clone().unwrap_or_default();
    for (i, url) in clip.urls.iter().enumerate() {
        list.set(i + 1, LuaUrl::parse(url))?;
    }
    list.set("is_cut", clip.cut)?;
    Ok(list)
}

#[cfg(test)]
mod tests;
