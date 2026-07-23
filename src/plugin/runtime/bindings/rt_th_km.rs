//! M4-T6/T7/T8: `rt` (runtime), `th` (theme), `km` (keymap)
//! globals for sync-context plugins.
//!
//! These are lightweight read-only views over the live application
//! state (AppConfig + AppState + KeybindingResolver). The bindings
//! re-build the table once per plugin load so they pick up
//! user-driven changes without round-tripping through the dispatcher.

use crate::app::state::AppState;
use crate::app::state::types::SortField;
use crate::config::theme::Theme;
use crate::keybindings::resolver::KeybindingResolver;
use mlua::Lua;

/// Build the `pairee.rt` table. M4-T6 surface:
///   rt.args.entries
///   rt.term.light
///   rt.mgr.sort_by / sort_reverse / show_hidden
///   rt.preview.wrap / tab_size
///   rt.tasks.count / running
pub fn build_rt_table(lua: &Lua, state: &AppState, cfg: &crate::config::AppConfig) -> mlua::Result<()> {
    let rt = lua.create_table()?;
    // rt.args (placeholder — populated from CLI args; we expose
    // entries as nil since most plugins run after arg parsing).
    let args = lua.create_table()?;
    args.set("entries", mlua::Value::Nil)?;
    args.set("cwd_file", mlua::Value::Nil)?;
    args.set("chooser_file", mlua::Value::Nil)?;
    rt.set("args", args)?;

    // rt.term
    let term = lua.create_table()?;
    // "light" if the theme name suggests a light theme.
    let is_light = matches!(
        cfg.settings.theme.to_lowercase().as_str(),
        "light" | "light-theme"
    );
    term.set("light", is_light)?;
    // rt.term.cell_size() → returns { w, h }. The terminal
    // backend does not currently expose its size outside of the
    // active draw pass, so we return zeros (plugins can detect
    // this and fall back to a sensible default). A live size
    // accessor is reserved for the next M4-T5 follow-up that
    // publishes the frame area through AppState.
    let cs = lua.create_table()?;
    cs.set("w", 0i64)?;
    cs.set("h", 0i64)?;
    term.set("cell_size", cs)?;
    rt.set("term", term)?;

    // rt.mgr
    let active_panel = state.get_active_panel();
    let mgr = lua.create_table()?;
    let sort_field = match active_panel.sort_field {
        SortField::Name => "name",
        SortField::Extension => "extension",
        SortField::Size => "size",
        SortField::Date => "date",
        SortField::Unsorted => "unsorted",
    };
    mgr.set("sort_by", sort_field)?;
    mgr.set("sort_reverse", active_panel.sort_reverse)?;
    mgr.set("show_hidden", active_panel.show_long_names)?;
    mgr.set("mouse_events", cfg.settings.mouse_support)?;
    rt.set("mgr", mgr)?;

    // rt.preview (no preview wrap setting on Settings today; we
    // expose 4-space tab and a 0-width default).
    let prev = lua.create_table()?;
    prev.set("wrap", false)?;
    prev.set("tab_size", 4i64)?;
    prev.set("max_width", 0i64)?;
    prev.set("max_height", 0i64)?;
    rt.set("preview", prev)?;

    // rt.tasks (placeholder; Pairee doesn't expose a per-task
    // counter via AppState — leave as zero counters).
    let tasks = lua.create_table()?;
    tasks.set("count", 0i64)?;
    tasks.set("running", 0i64)?;
    tasks.set("finished", 0i64)?;
    rt.set("tasks", tasks)?;

    lua.globals().set("rt", rt)?;
    Ok(())
}

/// Build the `pairee.th` table. M4-T7 surface: each leaf is a
/// table of Style-related fields. We expose a simplified map:
///   th.app, th.mgr, th.tabs, th.mode, th.indicator, th.status,
///   th.which, th.confirm, th.spot, th.notify, th.pick, th.input,
///   th.cmp, th.tasks, th.help
/// Each is a Lua table with `fg`, `bg`, `bold` etc. derived from
/// the `Theme` color strings.
pub fn build_th_table(lua: &Lua, theme: &Theme) -> mlua::Result<()> {
    let th = lua.create_table()?;
    let leaves: [(&str, &str, &str); 15] = [
        ("app",        &theme.popup_bg,   &theme.popup_fg),
        ("mgr",        &theme.panel_bg,   &theme.panel_fg),
        ("tabs",       &theme.popup_bg,   &theme.popup_fg),
        ("mode",       &theme.header_fg,  &theme.header_bg),
        ("indicator",  &theme.fkey_text_fg, &theme.fkey_bg),
        ("status",     &theme.cli_fg,     &theme.cli_bg),
        ("which",      &theme.popup_fg,   &theme.popup_bg),
        ("confirm",    &theme.popup_fg,   &theme.popup_bg),
        ("spot",       &theme.popup_fg,   &theme.popup_bg),
        ("notify",     &theme.popup_fg,   &theme.popup_bg),
        ("pick",       &theme.popup_fg,   &theme.popup_bg),
        ("input",      &theme.popup_fg,   &theme.popup_bg),
        ("cmp",        &theme.popup_fg,   &theme.popup_bg),
        ("tasks",      &theme.popup_fg,   &theme.popup_bg),
        ("help",       &theme.popup_fg,   &theme.popup_bg),
    ];
    for (name, bg, fg) in leaves.iter() {
        let leaf = lua.create_table()?;
        leaf.set("fg", fg.to_string())?;
        leaf.set("bg", bg.to_string())?;
        leaf.set("bold", false)?;
        leaf.set("italic", false)?;
        leaf.set("underline", false)?;
        th.set(*name, leaf)?;
    }
    lua.globals().set("th", th)?;
    Ok(())
}

/// Build the `pairee.km` table. M4-T8 + M5-pending surface:
///   km.default       — the global Pairee preset (single preset
///                      today; multi-preset support is M5+)
///   km.panels       — keymap active in the panels screen
///   km.viewer       — keymap active in the viewer screen
///   km.editor       — keymap active in the editor screen
///   km.terminal     — keymap active in the terminal screen
///   km.input        — keymap active inside an input prompt
///   km.which        — keymap active inside a which-prompt
///   km.manager      — keymap active inside the F11 plugin manager
pub fn build_km_table(lua: &Lua, _resolver: &KeybindingResolver) -> mlua::Result<()> {
    let km = lua.create_table()?;
    // Build the canonical Pairee defaults table once. Each layer
    // currently exposes the same defaults (Pairee uses a single
    // global preset today); the per-layer split exists so plugins
    // can `km.panels.MoveUp` style accesses without knowing about
    // future preset layering.
    let known_actions = [
        ("MoveUp", "Up"),
        ("MoveDown", "Down"),
        ("ChangePanel", "Tab"),
        ("Quit", "F10"),
        ("Help", "F1"),
        ("View", "F3"),
        ("Edit", "F4"),
        ("Copy", "F5"),
        ("Move", "F6"),
        ("MkDir", "F7"),
        ("Delete", "F8"),
    ];

    let layers = ["default", "panels", "viewer", "editor", "terminal", "input", "which", "manager"];
    for layer in layers.iter() {
        let table = lua.create_table()?;
        for (action, default_key) in known_actions.iter() {
            table.set(*action, *default_key)?;
        }
        km.set(*layer, table)?;
    }
    lua.globals().set("km", km)?;
    Ok(())
}
