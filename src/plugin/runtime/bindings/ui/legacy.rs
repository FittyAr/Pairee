//! M4-T1: legacy plain-table widget constructors. These are
//! preserved for back-compat with existing plugins but emit a
//! deprecation warning the first time they are called. The new
//! userdata-backed widget surface (in `elements/`, `style.rs`,
//! `preview.rs`) is the recommended API going forward.

use crate::plugin::manager::PluginRequest;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

static LEGACY_WARNED: AtomicBool = AtomicBool::new(false);

fn warn_once_legacy() {
    if !LEGACY_WARNED.swap(true, Ordering::SeqCst) {
        log::warn!(
            "pairee.ui.<X>(...) plain-table constructors are deprecated; use the \
             new userdata-backed API instead: \
             ui.Span(text):style(s):fg(\"red\"):bold(), \
             ui.Line(<span, span, ...>):style(s), \
             ui.Text(<line, line, ...>):style(s), \
             ui.Style():fg(\"red\"):bold():italic(). \
             (This warning is shown once per plugin process.)"
        );
    }
}

pub fn bind(lua: &mlua::Lua) -> mlua::Result<mlua::Table<'_>> {
    let ui = lua.create_table()?;

    ui.set(
        "Paragraph",
        lua.create_function(|lua_ctx, text: String| {
            warn_once_legacy();
            let t = lua_ctx.create_table()?;
            t.set("type", "Paragraph")?;
            t.set("text", text)?;
            Ok(t)
        })?,
    )?;

    ui.set(
        "Gauge",
        lua.create_function(|lua_ctx, (ratio, label): (f64, String)| {
            warn_once_legacy();
            let t = lua_ctx.create_table()?;
            t.set("type", "Gauge")?;
            t.set("ratio", ratio)?;
            t.set("label", label)?;
            Ok(t)
        })?,
    )?;

    ui.set(
        "List",
        lua.create_function(|lua_ctx, items: Vec<String>| {
            warn_once_legacy();
            let t = lua_ctx.create_table()?;
            t.set("type", "List")?;
            t.set("items", items)?;
            Ok(t)
        })?,
    )?;

    ui.set(
        "Table",
        lua.create_function(
            |lua_ctx, (headers, rows): (Vec<String>, Vec<Vec<String>>)| {
                warn_once_legacy();
                let t = lua_ctx.create_table()?;
                t.set("type", "Table")?;
                t.set("headers", headers)?;
                t.set("rows", rows)?;
                Ok(t)
            },
        )?,
    )?;

    ui.set(
        "Span",
        lua.create_function(|lua_ctx, (text, style): (String, String)| {
            warn_once_legacy();
            let t = lua_ctx.create_table()?;
            t.set("type", "Span")?;
            t.set("text", text)?;
            t.set("style", style)?;
            Ok(t)
        })?,
    )?;

    ui.set(
        "Line",
        lua.create_function(|lua_ctx, spans: Vec<mlua::Table>| {
            warn_once_legacy();
            let t = lua_ctx.create_table()?;
            t.set("type", "Line")?;
            t.set("spans", spans)?;
            Ok(t)
        })?,
    )?;

    Ok(ui)
}

// Re-export the request helper so the new `preview_widget` lives in
// its own module but shares the channel. Today unused; wired by
// `preview.rs` in M4-T1.
#[allow(dead_code)]
pub fn request_sender(_tx: &mpsc::UnboundedSender<PluginRequest>) {}

// Force-include PluginRequest so the import isn't dropped if the
// legacy module is the only thing using it.
const _: fn() = || {
    let _ = std::marker::PhantomData::<PluginRequest>;
};
