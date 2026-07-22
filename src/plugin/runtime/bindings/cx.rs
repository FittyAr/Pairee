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
use mlua::{UserData, UserDataFields, UserDataMethods};
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
    let current = lua.create_table()?;
    let cwd_str = active_panel.current_path.to_string_lossy().to_string();
    let cwd_url = Url::parse(&cwd_str);
    current.set("cwd", lua.create_userdata(cwd_url)?)?;
    current.set("files", active_panel.entries.len() as i64)?;
    current.set("offset", 0i64)?;
    current.set("cursor", active_panel.cursor_index as i64)?;

    // hovered: the entry at cursor_index
    if let Some(entry) = active_panel.entries.get(active_panel.cursor_index) {
        let url = Url::parse(&entry.path.to_string_lossy());
        let f = File::from_url(url);
        current.set("hovered", lua.create_userdata(f)?)?;
    }

    // selected: walk selection_order, build File[]
    let mut selected = Vec::new();
    for path in &active_panel.selection_order {
        if let Some(entry) = active_panel.entries.iter().find(|e| &e.path == path) {
            let url = Url::parse(&entry.path.to_string_lossy());
            let f = File::from_url(url);
            let ud = lua.create_userdata(f)?;
            selected.push(mlua::Value::UserData(ud));
        }
    }
    current.set("selected", selected)?;

    // helper (not in the roadmap but useful): the currently-
    // selected panel's `entries` for inspection. Stored under
    // cx.active.current._entries as a sequence of File[].
    let mut entries = Vec::new();
    for entry in &active_panel.entries {
        let url = Url::parse(&entry.path.to_string_lossy());
        let cha = crate::plugin::types::Cha::from_metadata(
            &std::fs::metadata(&entry.path).unwrap_or_else(|_| {
                std::fs::metadata(&active_panel.current_path)
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
    current.set("_entries", entries)?;

    active.set("current", current)?;

    // parent (other panel)
    let inactive_panel = state.get_passive_panel();
    let parent = lua.create_table()?;
    let parent_cwd = inactive_panel.current_path.to_string_lossy().to_string();
    let parent_url = Url::parse(&parent_cwd);
    parent.set("cwd", lua.create_userdata(parent_url)?)?;
    parent.set("files", inactive_panel.entries.len() as i64)?;
    active.set("parent", parent)?;

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
