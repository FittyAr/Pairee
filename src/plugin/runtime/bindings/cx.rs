//! M3 slim `cx` global — sync-only read access to live state.
//!
//! The full `cx` tree (per roadmap §5.D1) ships in M4 once the
//! sync/async VM split lands. M3 only exposes the two fields
//! the `fzf.pairee` acceptance plugin needs:
//!
//! - `cx.active.current.cwd` — the active panel's current working
//!   directory as a `Url` userdata.
//! - `cx.active.selected` — an array of `File` userdata
//!   representing the active panel's selection.
//!
//! The binding is *read-only* in M3. Plugins that want to mutate
//! the active panel go through `pairee.emit("cd"|"select"|...)`.

use mlua::UserData;
use std::path::PathBuf;

/// Snapshot of the live state we expose through `cx`. The main
/// loop builds one of these on every tick (just before draining
/// plugin requests) and stashes it on the `Runtime`. Sync
/// callbacks then read the snapshot from the `cx` global.
#[derive(Debug, Clone, Default)]
pub struct CxSnapshot {
    pub cwd: Option<PathBuf>,
    pub selected: Vec<PathBuf>,
}

impl UserData for CxSnapshot {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(_fields: &mut F) {}
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(_methods: &mut M) {}
}

/// Build a minimal `cx` table from the live state. Called by
/// the main loop before sync plugin callbacks run. M3 only
/// populates `active.current.cwd` and `active.selected`; M4 will
/// add the full tree (`tabs`, `tasks`, `yanked`, `input`,
/// `which`, `layer`).
pub fn build_cx_table(
    lua: &mlua::Lua,
    state: &crate::app::state::AppState,
) -> mlua::Result<()> {
    let cx = lua.create_table()?;
    let active = lua.create_table()?;
    let current = lua.create_table()?;
    let cwd = state
        .get_active_panel()
        .current_path
        .to_string_lossy()
        .to_string();
    current.set("cwd", cwd)?;
    active.set("current", current)?;
    // Selected: walk the selection_order and build File[].
    let panel = state.get_active_panel();
    let mut selected = Vec::new();
    for path in &panel.selection_order {
        if let Some(entry) = panel.entries.iter().find(|e| &e.path == path) {
            let url = crate::plugin::types::Url::parse(&entry.path.to_string_lossy());
            let f = crate::plugin::types::File::from_url(url);
            let ud = lua.create_userdata(f)?;
            selected.push(mlua::Value::UserData(ud));
        }
    }
    active.set("selected", selected)?;
    cx.set("active", active)?;
    lua.globals().set("cx", cx)?;
    Ok(())
}
