//! Lua binding for `pairee.which({cands, silent})`.
//!
//! Prompts the user to press one of the candidate keys and returns the
//! 1-based index of the selected candidate, or `nil` if the user cancels.
//!
//! In M0, the actual TUI popup is not yet wired; the dispatcher in
//! `process_plugin_requests` returns `nil` (cancel) so plugins that
//! migrate to this API get a deterministic placeholder. A future phase
//! will replace the M0 placeholder with a real popup.

use crate::plugin::manager::{PluginRequest, WhichCandidate};
use tokio::sync::mpsc;

pub fn bind(lua: &mlua::Lua, tx: mpsc::Sender<PluginRequest>) -> mlua::Result<mlua::Table<'_>> {
    let table = lua.create_table()?;

    let tx_which = tx;
    table.set(
        "which",
        lua.create_async_function(move |lua_ctx, opts: mlua::Table| {
            let tx = tx_which.clone();
            async move {
                if let Some(rt) = lua_ctx.app_data_ref::<crate::plugin::runtime::runtime::Runtime>() {
                    if rt.is_blocking() {
                        return Err(mlua::Error::RuntimeError(
                            "pairee.which cannot be called inside a sync block (re-entry guard)"
                                .to_string(),
                        ));
                    }
                }
                let silent = opts.get::<_, bool>("silent").unwrap_or(false);
                let candidates = match read_candidates(&opts) {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("pairee.which: failed to read candidates: {e}");
                        return Ok(mlua::Value::Nil);
                    }
                };
                if candidates.is_empty() {
                    log::warn!("pairee.which: no candidates provided");
                    return Ok(mlua::Value::Nil);
                }
                let (reply_tx, reply_rx) = tokio::sync::mpsc::unbounded_channel();
                if tx
                    .send(PluginRequest::WhichPrompt {
                        candidates,
                        silent,
                        reply_tx,
                    })
                    .await
                    .is_err()
                {
                    log::error!("pairee.which could not enqueue; main loop not running");
                    return Ok(mlua::Value::Nil);
                }
                let answer = crate::plugin::manager::recv_single(reply_rx).await;
                match answer {
                    Some(idx) => Ok(mlua::Value::Integer(idx as i64)),
                    None => Ok(mlua::Value::Nil),
                }
            }
        })?,
    )?;

    Ok(table)
}

/// Reads the `cands` table from a `pairee.which` opts table and converts
/// each entry into a `WhichCandidate`. Each `cand` may have a string `on`
/// (single key) or a sequence `on` (multiple equivalent keys), plus an
/// optional `desc` description.
fn read_candidates(opts: &mlua::Table) -> mlua::Result<Vec<WhichCandidate>> {
    let cands: mlua::Table = opts.get("cands")?;
    let mut out: Vec<WhichCandidate> = Vec::new();
    for pair in cands.sequence_values::<mlua::Table>() {
        let cand = pair?;
        let on_value: mlua::Value = cand.get("on")?;
        let on: Vec<String> = match on_value {
            mlua::Value::String(s) => vec![s.to_str()?.to_string()],
            mlua::Value::Table(t) => {
                let mut keys = Vec::new();
                for v in t.sequence_values::<mlua::String>() {
                    keys.push(v?.to_str()?.to_string());
                }
                keys
            }
            other => {
                log::warn!(
                    "pairee.which: `on` must be string or table, got {:?}",
                    other
                );
                return Err(mlua::Error::RuntimeError(
                    "pairee.which: invalid `on` value".to_string(),
                ));
            }
        };
        let desc: Option<String> = cand
            .get::<_, mlua::String>("desc")
            .ok()
            .map(|s| s.to_str().map(|s| s.to_string()))
            .transpose()?;
        out.push(WhichCandidate { on, desc });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    fn make_cands_table<'a>(
        lua: &'a Lua,
        entries: &'a [(&'a str, Option<&'a str>)],
    ) -> mlua::Result<mlua::Table<'a>> {
        let cands = lua.create_table()?;
        for (i, (on, desc)) in entries.iter().enumerate() {
            let cand = lua.create_table()?;
            cand.set("on", *on)?;
            if let Some(d) = desc {
                cand.set("desc", *d)?;
            }
            cands.set(i + 1, cand)?;
        }
        Ok(cands)
    }

    #[test]
    fn test_read_candidates_single_key() {
        let lua = Lua::new();
        let opts = lua.create_table().unwrap();
        let cands = make_cands_table(&lua, &[("a", Some("press a")), ("b", None)]).unwrap();
        opts.set("cands", cands).unwrap();
        let result = read_candidates(&opts).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].on, vec!["a".to_string()]);
        assert_eq!(result[0].desc.as_deref(), Some("press a"));
        assert_eq!(result[1].on, vec!["b".to_string()]);
        assert_eq!(result[1].desc, None);
    }

    #[test]
    fn test_read_candidates_multi_key() {
        let lua = Lua::new();
        let opts = lua.create_table().unwrap();
        let cands = lua.create_table().unwrap();
        let cand = lua.create_table().unwrap();
        let on = lua.create_table().unwrap();
        on.set(1, "a").unwrap();
        on.set(2, "<C-c>").unwrap();
        cand.set("on", on).unwrap();
        cand.set("desc", "either").unwrap();
        cands.set(1, cand).unwrap();
        opts.set("cands", cands).unwrap();

        let result = read_candidates(&opts).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].on, vec!["a".to_string(), "<C-c>".to_string()]);
        assert_eq!(result[0].desc.as_deref(), Some("either"));
    }

    #[test]
    fn test_read_candidates_missing_cands_key() {
        let lua = Lua::new();
        let opts = lua.create_table().unwrap();
        // no `cands` field — read_candidates should propagate the Lua
        // error rather than panicking.
        let result = read_candidates(&opts);
        assert!(result.is_err());
    }
}
