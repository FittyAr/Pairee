//! Lua binding for the structured `pairee.notify` API (M0).
//!
//! Existing plugins that call `pairee.app.notify(title, msg, level)`
//! continue to work; the structured form `pairee.notify({title=...,
//! content=..., level=..., timeout=...})` is the recommended new form.
//!
//! In M0 the timeout field is recorded in the log line but does not yet
//! drive a real auto-dismiss (M1 will wire the timer into the popup
//! manager). The title and content drive the popup rendering.

use crate::plugin::manager::{NotifyPayload, PluginRequest};
use tokio::sync::mpsc;

pub fn bind(lua: &mlua::Lua, tx: mpsc::Sender<PluginRequest>) -> mlua::Result<mlua::Table<'_>> {
    let table = lua.create_table()?;

    let tx_notify = tx;
    table.set(
        "notify",
        lua.create_async_function(move |_lua, opts: mlua::Table| {
            let tx = tx_notify.clone();
            async move {
                let title = match opts.get::<_, mlua::String>("title") {
                    Ok(s) => s.to_str()?.to_string(),
                    Err(_) => {
                        log::warn!("pairee.notify: missing or non-string `title`");
                        return Ok(mlua::Value::Nil);
                    }
                };
                let content = opts
                    .get::<_, mlua::String>("content")
                    .ok()
                    .map(|s| s.to_str().map(|s| s.to_string()))
                    .transpose()?
                    .unwrap_or_default();
                let level = opts
                    .get::<_, mlua::String>("level")
                    .ok()
                    .map(|s| s.to_str().map(|s| s.to_string()))
                    .transpose()?
                    .or_else(|| Some("info".to_string()));
                let timeout_secs = opts.get::<_, f64>("timeout").ok();

                let payload = NotifyPayload {
                    title,
                    content,
                    level,
                    timeout_secs,
                };
                if tx
                    .send(PluginRequest::NotifyStructured(payload))
                    .await
                    .is_err()
                {
                    log::error!("pairee.notify could not enqueue; main loop not running");
                }
                Ok(mlua::Value::Nil)
            }
        })?,
    )?;

    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify_payload_required_fields() {
        let p = NotifyPayload {
            title: "t".to_string(),
            content: "c".to_string(),
            level: Some("warn".to_string()),
            timeout_secs: Some(3.0),
        };
        assert_eq!(p.title, "t");
        assert_eq!(p.content, "c");
        assert_eq!(p.level.as_deref(), Some("warn"));
        assert_eq!(p.timeout_secs, Some(3.0));
    }
}
