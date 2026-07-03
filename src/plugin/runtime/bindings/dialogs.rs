//! Lua binding for the new structured dialog APIs (M0 dispatcher envelope).
//!
//! - `pairee.input({pos, title, value, obscure, realtime, debounce})` —
//!   dispatches a real input dialog request (`PluginRequest::InputDialog`).
//!   In M0 the dispatcher returns a placeholder `submitted` result with
//!   the default value; M1 will route the request to a TUI popup.
//! - `pairee.confirm({pos, title, body})` — dispatches a real confirm
//!   dialog request (`PluginRequest::ConfirmDialog`). In M0 the
//!   dispatcher returns a placeholder `false` (cancel).
//!
//! These are the new recommended entry points; the legacy
//! `pairee.app.confirm(title, msg)` and `pairee.app.input(title, default)`
//! stubs still work but emit a deprecation warning.

use crate::plugin::manager::{DialogPosition, PluginRequest, WhichCandidate};
#[cfg(test)]
use crate::plugin::manager::InputDialogResult;
use tokio::sync::mpsc;

pub fn bind(
    lua: &mlua::Lua,
    pairee: &mlua::Table<'_>,
    tx: mpsc::Sender<PluginRequest>,
) -> mlua::Result<()> {
    // `pairee.confirm({pos, title, body})` — real confirm dialog.
    let tx_confirm = tx.clone();
    pairee.set(
        "confirm",
        lua.create_async_function(move |lua_ctx, opts: mlua::Table| {
            let tx = tx_confirm.clone();
            async move {
                // M3 re-entry guard: `pairee.confirm`/`pairee.input`/`pairee.which`
                // cannot be called from inside a sync block (the
                // main thread is busy waiting for the dialog answer
                // and would deadlock).
                if let Some(rt) = lua_ctx.app_data_ref::<crate::plugin::runtime::runtime::Runtime>() {
                    if rt.is_blocking() {
                        return Err(mlua::Error::RuntimeError(
                            "pairee.confirm cannot be called inside a sync block (re-entry guard)"
                                .to_string(),
                        ));
                    }
                }
                // Extract the title as an owned `String` (or bail with a
                // false return if it is missing). The `to_str` helper
                // returns a `&str` borrowed from the Lua string, so we
                // eagerly own the bytes to release the borrow.
                let title_opt: Option<String> = match opts.get::<_, mlua::String>("title") {
                    Ok(s) => match s.to_str() {
                        Ok(cow) => Some(cow.to_string()),
                        Err(_) => None,
                    },
                    Err(_) => None,
                };
                let title = match title_opt {
                    Some(t) => t,
                    None => {
                        log::warn!("pairee.confirm: missing or non-string `title`");
                        return Ok(false);
                    }
                };
                let body: String = opts
                    .get::<_, mlua::String>("body")
                    .ok()
                    .and_then(|s| s.to_str().ok().map(|cow| cow.to_string()))
                    .unwrap_or_default();
                let position = read_position(&opts);
                let (reply_tx, reply_rx) = tokio::sync::mpsc::unbounded_channel();
                if tx
                    .send(PluginRequest::ConfirmDialog {
                        title,
                        msg: body,
                        position,
                        reply_tx,
                    })
                    .await
                    .is_err()
                {
                    log::error!("pairee.confirm could not enqueue; main loop not running");
                    return Ok(false);
                }
                Ok(crate::plugin::manager::recv_single(reply_rx).await)
            }
        })?,
    )?;

    // `pairee.input({pos, title, value, obscure, realtime, debounce})` —
    // real input dialog. The result is a table `{value, event}` where
    // `event` is 1 (submitted), 2 (cancelled), or 3 (typed).
    let tx_input = tx;
    pairee.set(
        "input",
        lua.create_async_function(move |lua_ctx, opts: mlua::Table| {
            let tx = tx_input.clone();
            async move {
                if let Some(rt) = lua_ctx.app_data_ref::<crate::plugin::runtime::runtime::Runtime>() {
                    if rt.is_blocking() {
                        return Err(mlua::Error::RuntimeError(
                            "pairee.input cannot be called inside a sync block (re-entry guard)"
                                .to_string(),
                        ));
                    }
                }
                let title_opt: Option<String> = match opts.get::<_, mlua::String>("title") {
                    Ok(s) => s.to_str().ok().map(|cow| cow.to_string()),
                    Err(_) => None,
                };
                let title = match title_opt {
                    Some(t) => t,
                    None => {
                        log::warn!("pairee.input: missing or non-string `title`");
                        return Ok(mlua::Value::Nil);
                    }
                };
                let value: String = opts
                    .get::<_, mlua::String>("value")
                    .ok()
                    .and_then(|s| s.to_str().ok().map(|cow| cow.to_string()))
                    .unwrap_or_default();
                let obscure = opts.get::<_, bool>("obscure").unwrap_or(false);
                let realtime = opts.get::<_, bool>("realtime").unwrap_or(false);
                let debounce_secs = opts.get::<_, f64>("debounce").unwrap_or(0.0);
                let position = read_position(&opts);

                let (reply_tx, reply_rx) = tokio::sync::mpsc::unbounded_channel();
                if tx
                    .send(PluginRequest::InputDialog {
                        title,
                        default: value.clone(),
                        position,
                        obscure,
                        realtime,
                        debounce_secs,
                        reply_tx,
                    })
                    .await
                    .is_err()
                {
                    log::error!("pairee.input could not enqueue; main loop not running");
                    return Ok(mlua::Value::Nil);
                }
                let result = crate::plugin::manager::recv_single(reply_rx).await;
                let lua_result = lua_ctx.create_table()?;
                lua_result.set("value", result.value)?;
                lua_result.set("event", result.event)?;
                Ok(mlua::Value::Table(lua_result))
            }
        })?,
    )?;

    Ok(())
}

/// Reads the optional `pos` field from a dialog opts table into a
/// `DialogPosition`. Returns `None` when the field is absent or malformed
/// so the dispatcher can fall back to a default-centered popup.
fn read_position(opts: &mlua::Table) -> Option<DialogPosition> {
    let pos: mlua::Table = opts.get("pos").ok()?;
    Some(DialogPosition {
        origin: pos
            .get::<_, mlua::String>(1)
            .ok()
            .map(|s| s.to_str().map(|s| s.to_string()))
            .transpose()
            .ok()
            .flatten()
            .unwrap_or_default(),
        x: pos.get::<_, i64>("x").ok().unwrap_or(0) as i32,
        y: pos.get::<_, i64>("y").ok().unwrap_or(0) as i32,
        w: pos.get::<_, u16>("w").ok().unwrap_or(0),
        h: pos.get::<_, u16>("h").ok().unwrap_or(0),
    })
}

// Suppress an unused-import warning for the WhichCandidate re-export
// (kept for future expansion to the `which` surface that lives in a
// sibling module).
#[allow(dead_code)]
const _KEEP_TYPES_IN_SCOPE: fn() = || {
    let _: fn() -> Option<WhichCandidate> = || None;
    let _: fn(DialogPosition) -> DialogPosition = std::convert::identity;
};

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    fn make_opts<'a>(
        lua: &'a Lua,
        title: &'a str,
        body: Option<&'a str>,
        pos_origin: Option<&'a str>,
    ) -> mlua::Table<'a> {
        let opts = lua.create_table().unwrap();
        opts.set("title", title).unwrap();
        if let Some(b) = body {
            opts.set("body", b).unwrap();
        }
        if let Some(origin) = pos_origin {
            let pos = lua.create_table().unwrap();
            pos.set(1, origin).unwrap();
            opts.set("pos", pos).unwrap();
        }
        opts
    }

    #[test]
    fn test_read_position_absent() {
        let lua = Lua::new();
        let opts = lua.create_table().unwrap();
        assert!(read_position(&opts).is_none());
    }

    #[test]
    fn test_read_position_with_origin() {
        let lua = Lua::new();
        let opts = make_opts(&lua, "t", None, Some("center"));
        let p = read_position(&opts).expect("position");
        assert_eq!(p.origin, "center");
        assert_eq!(p.x, 0);
        assert_eq!(p.w, 0);
    }

    #[test]
    fn test_input_dialog_result_serialization() {
        let r = InputDialogResult {
            value: "hello".to_string(),
            event: 3,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: InputDialogResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value, r.value);
        assert_eq!(back.event, r.event);
    }
}
