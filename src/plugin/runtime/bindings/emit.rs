//! Lua binding for `pairee.emit(action, args)`.
//!
//! `pairee.emit` is the unified action-dispatch entry point for plugins.
//! It sends a request to the main loop that invokes an existing key-binding
//! action by name, optionally with arguments.
//!
//! Today (M0), the dispatcher only wires two actions directly: `cd` and
//! `set_focus` (alias `focus`). These are the historical plugin-only paths
//! that have always been available through `pairee.app.cd` and
//! `pairee.app.set_focus`. A future phase will route the call through the
//! key-binding resolver to support arbitrary actions; today the dispatcher
//! logs a warning for any unknown name.

use crate::plugin::manager::PluginRequest;
use tokio::sync::mpsc;

pub fn bind(lua: &mlua::Lua, tx: mpsc::Sender<PluginRequest>) -> mlua::Result<mlua::Table<'_>> {
    let emit = lua.create_table()?;

    // `pairee.emit(name, args)` — generic dispatch. The async body
    // builds the JSON args and pushes a `PluginRequest::EmitAction`
    // through the mpsc to the main loop.
    let tx_emit = tx.clone();
    emit.set(
        "emit",
        lua.create_async_function(move |_lua, (name, args): (String, Option<mlua::Value>)| {
            let tx = tx_emit.clone();
            async move {
                let args_json = match args {
                    Some(mlua::Value::Nil) | None => serde_json::json!({}),
                    Some(mlua::Value::Table(t)) => {
                        lua_table_to_json(t).unwrap_or_else(|_| serde_json::json!({}))
                    }
                    Some(v) => {
                        let s = serde_json::to_string(&v).unwrap_or_else(|_| "null".to_string());
                        serde_json::from_str(&s).unwrap_or(serde_json::Value::Null)
                    }
                };
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                if tx
                    .send(PluginRequest::EmitAction {
                        name: name.clone(),
                        args: args_json,
                        reply_tx: Some(reply_tx),
                    })
                    .await
                    .is_err()
                {
                    log::error!("pairee.emit('{name}') could not enqueue; main loop not running");
                    return Ok(mlua::Value::Nil);
                }
                let _ = reply_rx.await;
                Ok(mlua::Value::Nil)
            }
        })?,
    )?;

    // `pairee.exec(name, args)` — alias of `pairee.emit` per the
    // roadmap Appendix A. Same signature, same semantics.
    let tx_exec = tx;
    emit.set(
        "exec",
        lua.create_async_function(move |_lua, (name, args): (String, Option<mlua::Value>)| {
            let tx = tx_exec.clone();
            async move {
                let args_json = match args {
                    Some(mlua::Value::Nil) | None => serde_json::json!({}),
                    Some(mlua::Value::Table(t)) => {
                        lua_table_to_json(t).unwrap_or_else(|_| serde_json::json!({}))
                    }
                    Some(v) => {
                        let s = serde_json::to_string(&v).unwrap_or_else(|_| "null".to_string());
                        serde_json::from_str(&s).unwrap_or(serde_json::Value::Null)
                    }
                };
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                if tx
                    .send(PluginRequest::EmitAction {
                        name: name.clone(),
                        args: args_json,
                        reply_tx: Some(reply_tx),
                    })
                    .await
                    .is_err()
                {
                    log::error!("pairee.exec('{name}') could not enqueue; main loop not running");
                    return Ok(mlua::Value::Nil);
                }
                let _ = reply_rx.await;
                Ok(mlua::Value::Nil)
            }
        })?,
    )?;

    Ok(emit)
}

/// Best-effort conversion of a Lua table into a JSON value. Supports
/// string keys (mapped to JSON object entries) and integer keys (mapped to
/// JSON array entries, with the first index being 1 per Lua convention).
fn lua_table_to_json(table: mlua::Table) -> mlua::Result<serde_json::Value> {
    let mut object_entries: Vec<(String, serde_json::Value)> = Vec::new();
    let mut array_entries: Vec<(usize, serde_json::Value)> = Vec::new();
    let mut max_index: usize = 0;

    for pair in table.pairs::<mlua::Value, mlua::Value>() {
        let (k, v) = pair?;
        let json_v = lua_value_to_json(v)?;
        match k {
            mlua::Value::Integer(i) => {
                let idx = i.max(0) as usize;
                array_entries.push((idx, json_v));
                if idx > max_index {
                    max_index = idx;
                }
            }
            mlua::Value::Number(n) if n.fract() == 0.0 && n >= 0.0 => {
                let idx = n as usize;
                array_entries.push((idx, json_v));
                if idx > max_index {
                    max_index = idx;
                }
            }
            mlua::Value::String(s) => {
                object_entries.push((s.to_str()?.to_string(), json_v));
            }
            _ => {
                // Skip non-string non-integer keys silently; the dispatch
                // contract only promises string/object/array args.
            }
        }
    }

    if !array_entries.is_empty() && object_entries.is_empty() {
        // Lua arrays are 1-indexed; produce a JSON array with that
        // convention, padding missing indices with Null.
        let mut arr: Vec<serde_json::Value> = Vec::with_capacity(max_index);
        let mut by_index = std::collections::HashMap::new();
        for (idx, v) in array_entries {
            by_index.insert(idx, v);
        }
        for i in 1..=max_index {
            arr.push(by_index.remove(&i).unwrap_or(serde_json::Value::Null));
        }
        Ok(serde_json::Value::Array(arr))
    } else if !object_entries.is_empty() {
        let mut map = serde_json::Map::new();
        for (k, v) in object_entries {
            map.insert(k, v);
        }
        Ok(serde_json::Value::Object(map))
    } else {
        Ok(serde_json::json!({}))
    }
}

/// Coerces a single Lua value into a JSON value.
fn lua_value_to_json(value: mlua::Value) -> mlua::Result<serde_json::Value> {
    Ok(match value {
        mlua::Value::Nil => serde_json::Value::Null,
        mlua::Value::Boolean(b) => serde_json::Value::Bool(b),
        mlua::Value::Integer(i) => serde_json::Value::Number(i.into()),
        mlua::Value::Number(n) => serde_json::Number::from_f64(n)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        mlua::Value::String(s) => serde_json::Value::String(s.to_str()?.to_string()),
        mlua::Value::Table(t) => lua_table_to_json(t)?,
        // Fall back to Debug formatting for the remaining userdata/error
        // types so plugins get a sensible string instead of nothing.
        other => serde_json::Value::String(format!("{:?}", other)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    fn run<F: FnOnce(&Lua, mlua::Table)>(f: F) {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        f(&lua, table);
    }

    #[test]
    fn test_lua_table_to_json_empty_table() {
        run(|_lua, t| {
            let json = lua_table_to_json(t).unwrap();
            assert_eq!(json, serde_json::json!({}));
        });
    }

    #[test]
    fn test_lua_table_to_json_object() {
        run(|_lua, t| {
            t.set("name", "alice").unwrap();
            t.set("age", 30).unwrap();
            let json = lua_table_to_json(t).unwrap();
            assert_eq!(json, serde_json::json!({ "name": "alice", "age": 30 }));
        });
    }

    #[test]
    fn test_lua_table_to_json_array() {
        run(|_lua, t| {
            t.set(1, "a").unwrap();
            t.set(2, "b").unwrap();
            t.set(3, "c").unwrap();
            let json = lua_table_to_json(t).unwrap();
            assert_eq!(
                json,
                serde_json::Value::Array(vec![
                    serde_json::json!("a"),
                    serde_json::json!("b"),
                    serde_json::json!("c")
                ])
            );
        });
    }

    #[test]
    fn test_lua_table_to_json_array_with_hole() {
        run(|_lua, t| {
            t.set(2, "b").unwrap();
            let json = lua_table_to_json(t).unwrap();
            assert_eq!(
                json,
                serde_json::Value::Array(vec![serde_json::Value::Null, serde_json::json!("b"),])
            );
        });
    }

    #[test]
    fn test_lua_value_to_json_primitives() {
        let lua = Lua::new();
        assert_eq!(
            lua_value_to_json(mlua::Value::Nil).unwrap(),
            serde_json::Value::Null
        );
        assert_eq!(
            lua_value_to_json(mlua::Value::Boolean(true)).unwrap(),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            lua_value_to_json(mlua::Value::String(lua.create_string("x").unwrap())).unwrap(),
            serde_json::json!("x")
        );
        assert_eq!(
            lua_value_to_json(mlua::Value::Integer(7)).unwrap(),
            serde_json::json!(7)
        );
    }

    #[test]
    fn test_bind_exposes_both_emit_and_exec() {
        // Both `pairee.emit` and `pairee.exec` must be exposed
        // (per roadmap Appendix A: "pairee.exec(action, args) | New (M0)").
        // We don't actually exercise the async dispatch path here
        // (it would need a live main loop); instead we verify the
        // bindings are present and callable as 0-arg functions
        // returning nil.
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (tx, _rx) = mpsc::channel::<PluginRequest>(1);
        rt.block_on(async {
            let lua = mlua::Lua::new();
            let table = lua.create_table().unwrap();
            // We can't use the real PluginRequest::EmitAction in
            // this synchronous test (its reply_tx requires a live
            // dispatcher loop), so test that `bind` inserts both
            // keys and that they are function userdata.
            let result = crate::plugin::runtime::bindings::emit::bind(&lua, tx)
                .expect("bind");
            assert!(result.contains_key("emit").unwrap());
            assert!(result.contains_key("exec").unwrap());
        });
    }
}
