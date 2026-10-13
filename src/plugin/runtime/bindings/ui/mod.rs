//! `pairee.ui`: widgets a previewer returns from `peek` (or passes to
//! `pairee.preview_widget`). They are userdata built in Rust from plain
//! data ([`crate::app::state::popup::plugin_widget`]); the UI layer maps
//! them to ratatui when it draws the quick view.
//!
//! * text: `Span`, `Line`, `Text` (`Text.parse` for ANSI), `Style`;
//! * widgets: `Paragraph`, `List`, `Table` / `Row`, `Gauge`, `Border`;
//! * layout: `Layout`, `Constraint`;
//! * constants: `Align`, `Wrap`, `Edge`, `Border.*`, `Layout.*`.

pub mod convert;
mod layout;
mod style;
mod text;
mod widgets;

pub use convert::to_renderable;

use crate::app::state::popup::plugin_widget::WStyle;
use mlua::{Lua, Table};

/// Builds the `pairee.ui` table.
pub fn bind(lua: &Lua) -> mlua::Result<Table> {
    let ui = lua.create_table()?;
    text::bind(lua, &ui)?;
    widgets::bind(lua, &ui)?;
    layout::bind(lua, &ui)?;
    ui.set("Style", lua.create_function(|_, ()| Ok(WStyle::default()))?)?;
    let constants = |pairs: &[(&str, &str)]| -> mlua::Result<Table> {
        let table = lua.create_table()?;
        for (name, value) in pairs {
            table.set(*name, *value)?;
        }
        Ok(table)
    };
    ui.set(
        "Align",
        constants(&[("LEFT", "left"), ("CENTER", "center"), ("RIGHT", "right")])?,
    )?;
    ui.set(
        "Wrap",
        constants(&[("NO", "no"), ("YES", "yes"), ("TRIM", "trim")])?,
    )?;
    Ok(ui)
}
