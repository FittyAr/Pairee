//! `ui` subdirectory entry point. Wires the legacy 6 plain-table
//! constructors, the new userdata-backed `Span`/`Line`/`Text`
//! widgets, the `Style`/`Color` userdata, the geometry primitives
//! (Rect/Constraint/Pad/Pos/Align/Wrap/Edge/Layout), and the
//! `pairee.preview_widget` bridge to the preview pane.

pub mod elements;
pub mod geometry;
pub mod legacy;
pub mod preview;
pub mod style;

use crate::plugin::manager::PluginRequest;
use std::sync::Arc;
use tokio::sync::mpsc::error::TrySendError;

/// Callback used by `preview_widget` to send a request to the
/// main loop. The mpsc sender shape is the caller's choice
/// (bounded or unbounded) — `bind` accepts any `Fn(PluginRequest)
/// -> Result<(), TrySendError<PluginRequest>>`.
pub type SendFn = Arc<dyn Fn(PluginRequest) -> Result<(), TrySendError<PluginRequest>> + Send + Sync>;

// `preview::bind` already requires the `Fn(PluginRequest) -> ...`
// shape. The test for `ui::bind` uses it directly.

/// Bind the entire `pairee.ui` namespace on a `pairee` table, plus
/// the `pairee.preview_widget` bridge on the same `pairee` table.
///
/// NOTE: the `pairee` table parameter must already be a `mlua::Table`.
pub fn bind(
    lua: &mlua::Lua,
    pairee: &mlua::Table<'_>,
    tx: SendFn,
) -> mlua::Result<()> {
    let ui = legacy::bind(lua)?;

    // The new widget userdata constructors overwrite the legacy
    // keys (the legacy ones were plain tables; the new ones are
    // userdata-backed). The legacy constructors in `legacy.rs`
    // log a deprecation warning.
    elements::span::bind(lua, &ui)?;
    elements::line::bind(lua, &ui)?;
    elements::text::bind(lua, &ui)?;
    elements::paragraph::bind(lua, &ui)?;
    elements::list::bind(lua, &ui)?;
    elements::gauge::bind(lua, &ui)?;
    elements::cell::bind(lua, &ui)?;
    elements::table::bind(lua, &ui)?;
    elements::table::bind_row(lua, &ui)?;
    style::bind(lua, &ui)?;

    // M4-T3 geometry primitives.
    geometry::bind_rect(lua, &ui)?;
    geometry::bind_constraint(lua, &ui)?;
    geometry::bind_pad(lua, &ui)?;
    geometry::bind_pos(lua, &ui)?;
    geometry::bind_align(lua, &ui)?;
    geometry::bind_wrap(lua, &ui)?;
    geometry::bind_edge(lua, &ui)?;
    geometry::bind_layout(lua, &ui)?;

    // Hide the metatable on `ui` so plugins can't accidentally
    // override our types.
    let mt = lua.create_table()?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    ui.set_metatable(Some(mt));

    // Install `pairee.ui = <our ui table>`. The legacy module
    // returns the table; we re-bind it.
    pairee.set("ui", ui)?;

    // `pairee.preview_widget(opts, widget)` — bridge from widget
    // userdata to the existing preview pane.
    preview::bind(lua, pairee, tx)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_bind_registers_ui_keys() {
        let lua = Lua::new();
        let pairee = lua.create_table().unwrap();
        let send_fn: SendFn = Arc::new(|_| Ok(()));
        bind(&lua, &pairee, send_fn).unwrap();
        // ui table must be present
        let ui: mlua::Table = pairee.get("ui").unwrap();
        for key in [
            "Paragraph",
            "Gauge",
            "List",
            "Table",
            "Span",
            "Line",
            "Text",
            "Style",
            "Color",
        ] {
            assert!(ui.contains_key(key).unwrap(), "ui missing key {key}");
        }
        // preview_widget must be a function on the central pairee
        assert!(pairee.contains_key("preview_widget").unwrap());
    }
}
