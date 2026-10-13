//! `pairee.rt`: read-only facts about the running Pairee and this plugin.
//!
//! ```lua
//! rt.version, rt.api, rt.os, rt.family
//! rt.plugin.name, rt.plugin.dir, rt.plugin.data_dir, rt.plugin.trusted, rt.plugin.secure_mode
//! rt.mgr.show_hidden, rt.mgr.sort_by, rt.mgr.sort_reverse
//! rt.preview.image, rt.preview.max_bytes
//! rt.term.size()   --> cols, rows (live; nil without a terminal)
//! ```
//!
//! The preferences are those of the configuration the plugins were loaded
//! with (they reload when the user changes plugin settings, trust or
//! Secure Mode). Built on first access (`runtime::composer`).

use crate::plugin::load_options::PluginLoadOptions;
use mlua::{Lua, Table};
use std::path::Path;

fn table(lua: &Lua, entries: Vec<(&str, mlua::Value)>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    for (key, value) in entries {
        t.set(key, value)?;
    }
    Ok(t)
}

/// Builds `pairee.rt` for the plugin in `plugin_dir` loaded with
/// `options`; `data_dir` is its private data directory.
pub fn bind(
    lua: &Lua,
    options: &PluginLoadOptions,
    plugin_dir: &Path,
    data_dir: &Path,
) -> mlua::Result<Table> {
    use mlua::IntoLua;
    let host = &options.host;
    let text = |s: &str| s.into_lua(lua);
    let path = |p: &Path| p.to_string_lossy().into_owned().into_lua(lua);
    let plugin = table(
        lua,
        vec![
            ("name", text(&options.name)?),
            ("dir", path(plugin_dir)?),
            ("data_dir", path(data_dir)?),
            ("trusted", options.trusted.into_lua(lua)?),
            ("secure_mode", options.secure_mode.into_lua(lua)?),
        ],
    )?;
    let mgr = table(
        lua,
        vec![
            ("show_hidden", host.show_hidden.into_lua(lua)?),
            ("sort_by", text(&host.sort_by)?),
            ("sort_reverse", host.sort_reverse.into_lua(lua)?),
        ],
    )?;
    let preview = table(
        lua,
        vec![
            ("image", host.image_preview.into_lua(lua)?),
            (
                "max_bytes",
                crate::app::state::quick_view::load::QUICK_VIEW_MAX_BYTES.into_lua(lua)?,
            ),
        ],
    )?;
    let term = lua.create_table()?;
    term.set(
        "size",
        lua.create_function(|_, ()| Ok(crossterm::terminal::size().ok().unzip()))?,
    )?;
    table(
        lua,
        vec![
            ("version", text(env!("CARGO_PKG_VERSION"))?),
            (
                "api",
                text(crate::plugin::runtime::api_version::LUA_API_VERSION)?,
            ),
            ("os", text(std::env::consts::OS)?),
            ("family", text(std::env::consts::FAMILY)?),
            ("plugin", mlua::Value::Table(plugin)),
            ("mgr", mlua::Value::Table(mgr)),
            ("preview", mlua::Value::Table(preview)),
            ("term", mlua::Value::Table(term)),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rt_shows_host_and_plugin_facts() {
        let lua = Lua::new();
        let mut options = PluginLoadOptions::new("demo", false, true);
        options.host.sort_by = "size".into();
        options.host.show_hidden = true;
        let rt = bind(&lua, &options, Path::new("p"), Path::new("d")).unwrap();
        lua.globals().set("rt", rt).unwrap();
        let (name, trusted, secure, sort, hidden, api): (String, bool, bool, String, bool, String) =
            lua.load(
                "return rt.plugin.name, rt.plugin.trusted, rt.plugin.secure_mode, \
                 rt.mgr.sort_by, rt.mgr.show_hidden, rt.api",
            )
            .eval()
            .unwrap();
        assert_eq!((name.as_str(), sort.as_str()), ("demo", "size"));
        assert!(!trusted && secure && hidden);
        assert_eq!(api, crate::plugin::runtime::api_version::LUA_API_VERSION);
        let size: mlua::MultiValue = lua.load("return rt.term.size()").eval().unwrap();
        assert!(size.len() <= 2);
    }
}
