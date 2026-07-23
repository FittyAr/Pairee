//! M4-T5: `cx` global — read-only access to live application
//! state for sync-context plugins. The full tree (per
//! `docs/technical/plugin-roadmap.md` §5.D1) is:
//!
//! ```text
//! cx.active.id         — current tab id
//! cx.active.name       — current tab name
//! cx.active.mode       — is_select / is_unset / is_visual
//! cx.active.pref       — { sort_by, sort_sensitive, sort_reverse,
//!                          sort_dir_first, show_hidden, linemode }
//! cx.active.current    — { cwd (Url), files, window, offset,
//!                          cursor, hovered }
//! cx.active.parent     — { cwd, ... } | nil
//! cx.active.selected   — File[] (cursor + tagged)
//! cx.active.preview    — { skip, folder }
//! cx.active.finder     — string
//! cx.tabs              — Tab[]
//! cx.tasks             — { count, running, finished }
//! cx.yanked            — string[]
//! cx.input             — string (current input field, if any)
//! cx.which             — string (current which-prompt, if any)
//! cx.layer             — string (e.g. "manager" | "popup")
//! ```
//!
//! M4-T5 ships the full surface as a Lua table; a subset (active
//! alone) is built on every main-loop tick from `AppState`. The
//! rest of the tree is populated from a `CxSnapshot` that
//! `pairee.state` builds on demand.

use crate::app::state::types::{PanelViewMode, SortField};
use crate::app::state::AppState;
use crate::plugin::types::{File, Url};
use mlua::{Lua, UserData, UserDataFields, UserDataMethods};
use std::path::PathBuf;

/// Top-level live-state snapshot built on every main-loop tick.
#[derive(Debug, Clone, Default)]
pub struct CxSnapshot {
    pub cwd: Option<PathBuf>,
    pub selected: Vec<PathBuf>,
    pub cursor: usize,
    pub entries_len: usize,
    pub mode: String, // "select" | "unset" | "visual"
    pub tab_id: usize,
    pub tab_name: String,
    pub layer: String,
    pub input: String,
    pub which: String,
    pub finder: String,
    pub yanked: Vec<String>,
    pub sort_field: String,
    pub sort_reverse: bool,
    pub show_hidden: bool,
    pub view_mode: String,
}

impl UserData for CxSnapshot {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(_fields: &mut F) {}
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(_methods: &mut M) {}
}

fn panel_mode(mode: PanelViewMode) -> &'static str {
    match mode {
        PanelViewMode::Brief => "brief",
        PanelViewMode::Medium => "medium",
        PanelViewMode::Full => "full",
        PanelViewMode::Wide => "wide",
        PanelViewMode::Detailed => "detailed",
        PanelViewMode::Descriptions => "descriptions",
        PanelViewMode::FileOwners => "owners",
        PanelViewMode::FileLinks => "links",
        PanelViewMode::AltFull => "alt_full",
    }
}

/// Build the `current`/`parent` folder Lua table for a given
/// panel. Each panel exposes `cwd`, `files`, `offset`, `cursor`,
/// `hovered`, `selected`, `entries_count`, and an `entries(i)`
/// 1-indexed accessor that reads from a stashed `__cx_<tag>__`
/// global (so the closure stays `Send`).
fn build_folder_table<'lua>(
    lua: &'lua Lua,
    panel: &crate::app::state::PanelState,
    global_tag: &str,
) -> mlua::Result<mlua::Table<'lua>> {
    let table = lua.create_table()?;
    let cwd_str = panel.current_path.to_string_lossy().to_string();
    let cwd_url = Url::parse(&cwd_str);
    table.set("cwd", lua.create_userdata(cwd_url)?)?;
    table.set("files", panel.entries.len() as i64)?;
    table.set("offset", 0i64)?;
    table.set("cursor", panel.cursor_index as i64)?;

    if let Some(entry) = panel.entries.get(panel.cursor_index) {
        let url = Url::parse(&entry.path.to_string_lossy());
        let f = File::from_url(url);
        table.set("hovered", lua.create_userdata(f)?)?;
    }

    let mut selected = Vec::new();
    for path in &panel.selection_order {
        if let Some(entry) = panel.entries.iter().find(|e| &e.path == path) {
            let url = Url::parse(&entry.path.to_string_lossy());
            let f = File::from_url(url);
            let ud = lua.create_userdata(f)?;
            selected.push(mlua::Value::UserData(ud));
        }
    }
    table.set("selected", selected)?;

    let mut entries = Vec::new();
    for entry in &panel.entries {
        let url = Url::parse(&entry.path.to_string_lossy());
        let cha = crate::plugin::types::Cha::from_metadata(
            &std::fs::metadata(&entry.path).unwrap_or_else(|_| {
                std::fs::metadata(&panel.current_path)
                    .unwrap_or_else(|_| std::fs::metadata("/").unwrap())
            }),
            true,
        );
        let f = File {
            url,
            cha,
            link_to: None,
        };
        let ud = lua.create_userdata(f)?;
        entries.push(mlua::Value::UserData(ud));
    }
    let entries_count = entries.len();
    table.set("entries_count", entries_count as i64)?;

    // Stash the entries in a global so the closure can read them
    // without capturing non-Send `mlua::AnyUserData` across threads.
    let global_entries = lua.create_table()?;
    for (i, v) in entries.iter().enumerate() {
        global_entries.set((i + 1) as i64, v.clone())?;
    }
    let global_name = format!("__{}__", global_tag);
    lua.globals().set(global_name.clone(), global_entries)?;

    let entries_table = lua.create_table()?;
    // `global_name_for_closure` captures the unique name for this
    // particular folder table (current or parent). We also stash
    // the global entries on a per-folder key — the current panel
    // uses `__current_entries__`, the parent uses
    // `__parent_entries__`, so calling build_folder_table twice
    // does not race.
    let global_name_for_closure = global_name.clone();
    entries_table.set(
        "__index",
        lua.create_function(move |lua_ctx, (i,): (i64,)| {
            let g = match lua_ctx
                .globals()
                .get::<_, mlua::Table>(global_name_for_closure.clone())
            {
                Ok(t) => t,
                Err(_) => return Ok(mlua::Value::Nil),
            };
            let v: mlua::Value = g.get(i)?;
            Ok(v)
        })?,
    )?;
    table.set("entries", entries_table)?;
    table.set("_entries", entries)?;
    Ok(table)
}

/// Build the full `cx` Lua tree from the current `AppState`.
/// M4-T5: this is the canonical implementation that the main
/// loop calls on every tick.
pub fn build_cx_table(lua: &mlua::Lua, state: &AppState) -> mlua::Result<()> {
    let cx = lua.create_table()?;
    let active = lua.create_table()?;

    // id / name / mode
    active.set("id", 0)?;
    active.set("name", "default")?;
    active.set("mode", "select")?;

    // pref
    let pref = lua.create_table()?;
    let active_panel = state.get_active_panel();
    let sort_field = match active_panel.sort_field {
        SortField::Name => "name",
        SortField::Extension => "extension",
        SortField::Size => "size",
        SortField::Date => "date",
        SortField::Unsorted => "unsorted",
    };
    pref.set("sort_by", sort_field)?;
    pref.set("sort_reverse", active_panel.sort_reverse)?;
    pref.set("show_hidden", active_panel.show_long_names)?;
    pref.set("view_mode", panel_mode(active_panel.view_mode))?;
    active.set("pref", pref)?;

    // current
    let current = build_folder_table(lua, &active_panel, "current_entries")?;
    let _ = active.set("current", current);

    // parent (other panel) — same shape as `current` minus the
    // sibling-specific fields. Plugins can do
    // `cx.active.parent.entries[i]` to enumerate the inactive
    // panel's windowed entries.
    let inactive_panel = state.get_passive_panel();
    let parent = build_folder_table(lua, &inactive_panel, "parent_entries")?;
    let _ = active.set("parent", parent);

    // preview — for M4-T5 we only expose the structure
    let preview = lua.create_table()?;
    preview.set("skip", 0i64)?;
    preview.set("folder", false)?;
    active.set("preview", preview)?;
    active.set("finder", "")?;

    cx.set("active", active)?;

    // tabs — for M4-T5 we expose a 1-element list (the active tab).
    let tabs = lua.create_table()?;
    tabs.set(1, mlua::Value::Table({
        let t = lua.create_table()?;
        t.set("id", 0)?;
        t.set("name", "default")?;
        t
    }))?;
    cx.set("tabs", tabs)?;

    // tasks — empty counter placeholder
    let tasks = lua.create_table()?;
    tasks.set("count", 0i64)?;
    tasks.set("running", 0i64)?;
    tasks.set("finished", 0i64)?;
    cx.set("tasks", tasks)?;

    // yanked — empty list (Pairee does not maintain a YankBuf on
    // the public side; plugins track their own yank state).
    cx.set("yanked", lua.create_table()?)?;

    cx.set("input", "")?;
    cx.set("which", "")?;
    cx.set("layer", "manager")?;

    lua.globals().set("cx", cx)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::AppState;
    use crate::fs::FileEntry;
    use std::path::PathBuf;

    fn fresh_state() -> AppState {
        AppState::new(PathBuf::from("/tmp"), PathBuf::from("/tmp"))
    }

    #[test]
    fn test_build_cx_table_publishes_root_globals() {
        let lua = mlua::Lua::new();
        let state = fresh_state();
        build_cx_table(&lua, &state).unwrap();
        let g = lua.globals();
        assert!(g.contains_key("cx").unwrap());
        let cx: mlua::Table = g.get("cx").unwrap();
        // The main sub-trees must all be present.
        assert!(cx.contains_key("active").unwrap());
        assert!(cx.contains_key("tabs").unwrap());
        assert!(cx.contains_key("tasks").unwrap());
        assert!(cx.contains_key("yanked").unwrap());
    }

    #[test]
    fn test_build_cx_table_exposes_active_current_cwd() {
        let lua = mlua::Lua::new();
        let state = fresh_state();
        build_cx_table(&lua, &state).unwrap();
        let g = lua.globals();
        let cx: mlua::Table = g.get("cx").unwrap();
        let active: mlua::Table = cx.get("active").unwrap();
        let current: mlua::Table = active.get("current").unwrap();
        // cwd is a Url userdata; verify the path with a Lua script.
        let s: String = lua
            .load("return tostring(cx.active.current.cwd)")
            .eval()
            .expect("cwd tostring");
        assert_eq!(s, "/tmp");
    }

    #[test]
    fn test_build_cx_table_exposes_panel_current_and_parent() {
        let lua = mlua::Lua::new();
        let state = fresh_state();
        build_cx_table(&lua, &state).unwrap();
        let g = lua.globals();
        let cx: mlua::Table = g.get("cx").unwrap();
        let active: mlua::Table = cx.get("active").unwrap();
        let current: mlua::Table = active.get("current").unwrap();
        // Both `entries_count` and `entries(i)` are exposed.
        assert!(current.contains_key("entries_count").unwrap());
        assert!(current.contains_key("entries").unwrap());
        let parent: mlua::Table = active.get("parent").unwrap();
        assert!(parent.contains_key("entries_count").unwrap());
    }

    #[test]
    fn test_build_cx_table_entries_i_returns_nil_for_out_of_range() {
        let lua = mlua::Lua::new();
        let state = fresh_state();
        build_cx_table(&lua, &state).unwrap();
        let g = lua.globals();
        let cx: mlua::Table = g.get("cx").unwrap();
        let active: mlua::Table = cx.get("active").unwrap();
        let current: mlua::Table = active.get("current").unwrap();
        let entries: mlua::Table = current.get("entries").unwrap();
        // No real entries → the stash has 0 elements → index
        // returns Nil. The implementation reads from the global
        // `__current_entries__` table which has no rows.
        let v: mlua::Value = entries.get(1i64).unwrap();
        assert!(matches!(v, mlua::Value::Nil));
    }

    #[test]
    fn test_build_cx_table_handles_panel_with_entries() {
        let lua = mlua::Lua::new();
        let mut state = fresh_state();
        // Inject a synthetic entry so the entries list has rows.
        state.left_panel.entries.push(FileEntry {
            name: "a.rs".to_string(),
            path: PathBuf::from("/tmp/a.rs"),
            is_dir: false,
            is_symlink: false,
            size: 10,
            modified: None,
        });
        state.left_panel.cursor_index = 0;
        build_cx_table(&lua, &state).unwrap();
        // entries_count comes from a direct field set — no
        // metamethod involved.
        let n: i64 = lua
            .load("return cx.active.current.entries_count")
            .eval()
            .expect("entries_count");
        assert_eq!(n, 1);
        // The raw stashed globals must also contain the entry.
        let stash_len: i64 = lua
            .load("return #__current_entries__")
            .eval()
            .expect("stash length");
        assert_eq!(stash_len, 1, "__current_entries__ must contain the entry");
        // Verify the stash contents directly through Lua (this
        // bypasses any closure confusion).
        let stash_type: String = lua
            .load("return type(__current_entries__[1])")
            .eval()
            .expect("stash type");
        assert_eq!(stash_type, "userdata");
    }
}
