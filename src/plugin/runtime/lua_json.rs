//! The one Lua ⇄ JSON converter of the plugin runtime.
//!
//! Every binding that turns plugin values into JSON (`pairee.emit`,
//! `pairee.ps.pub`, `pairee.json_encode`) or JSON into Lua
//! (`pairee.json_decode`, hook payloads) goes through this module, which
//! relies on mlua's serde support (`LuaSerdeExt`) instead of a hand-written
//! walker.
//!
//! Plugin input is untrusted, so the conversion is bounded first:
//!
//! * nesting deeper than [`MAX_DEPTH`] or more than [`MAX_NODES`] values
//!   are refused (a self-referencing table hits the depth limit);
//! * a table is either a **dense** sequence (keys exactly `1..=n`, which
//!   becomes a JSON array) or a map with string keys (a JSON object). Sparse
//!   or mixed tables such as `{[2^60] = 1}` are refused instead of being
//!   padded, so a plugin cannot make the host allocate a huge array.

use mlua::{Lua, LuaSerdeExt, Table, Value};

/// Deepest table nesting accepted from a plugin.
pub const MAX_DEPTH: usize = 32;

/// Most values (scalars and tables) accepted in one conversion.
pub const MAX_NODES: usize = 100_000;

/// Converts a plugin value into JSON, or explains why it is refused.
pub fn to_json(lua: &Lua, value: Value) -> mlua::Result<serde_json::Value> {
    let mut nodes = 0usize;
    check_value(&value, 0, &mut nodes)?;
    lua.from_value(value)
}

/// Converts plugin arguments into JSON: `nil` becomes `{}` so dispatchers
/// can always read keys, anything else goes through [`to_json`].
pub fn args_to_json(lua: &Lua, value: Option<Value>) -> mlua::Result<serde_json::Value> {
    match value {
        None | Some(Value::Nil) => Ok(serde_json::json!({})),
        Some(v) => to_json(lua, v),
    }
}

/// Converts JSON produced by the host into a Lua value.
pub fn from_json<'lua>(lua: &'lua Lua, value: &serde_json::Value) -> mlua::Result<Value<'lua>> {
    lua.to_value(value)
}

fn refuse(reason: &str) -> mlua::Error {
    mlua::Error::RuntimeError(format!("value cannot be converted to JSON: {reason}"))
}

fn check_value(value: &Value, depth: usize, nodes: &mut usize) -> mlua::Result<()> {
    *nodes += 1;
    if *nodes > MAX_NODES {
        return Err(refuse(&format!("more than {MAX_NODES} values")));
    }
    match value {
        Value::Table(table) => check_table(table, depth + 1, nodes),
        Value::Nil
        | Value::Boolean(_)
        | Value::Integer(_)
        | Value::Number(_)
        | Value::String(_) => Ok(()),
        other => Err(refuse(&format!("unsupported type `{}`", other.type_name()))),
    }
}

fn check_table(table: &Table, depth: usize, nodes: &mut usize) -> mlua::Result<()> {
    if depth > MAX_DEPTH {
        return Err(refuse(&format!("nested deeper than {MAX_DEPTH} levels")));
    }
    let len = table.raw_len();
    let mut count = 0usize;
    for pair in table.clone().pairs::<Value, Value>() {
        let (key, value) = pair?;
        count += 1;
        match key {
            Value::String(_) if len == 0 => {}
            Value::Integer(i) if i >= 1 && (i as usize) <= len => {}
            Value::String(_) | Value::Integer(_) | Value::Number(_) => {
                return Err(refuse(
                    "tables must be dense lists (keys 1..n) or have only string keys",
                ));
            }
            other => return Err(refuse(&format!("unsupported key `{}`", other.type_name()))),
        }
        check_value(&value, depth, nodes)?;
    }
    if count != len && len > 0 {
        return Err(refuse("list has holes"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(lua: &Lua, code: &str) -> mlua::Result<serde_json::Value> {
        let v: Value = lua.load(code).eval()?;
        to_json(lua, v)
    }

    #[test]
    fn objects_lists_and_scalars_convert() {
        let lua = Lua::new();
        assert_eq!(
            eval(&lua, r#"return { name = "alice", age = 30 }"#).unwrap(),
            serde_json::json!({ "name": "alice", "age": 30 })
        );
        assert_eq!(
            eval(&lua, r#"return { "a", "b", { x = true } }"#).unwrap(),
            serde_json::json!(["a", "b", { "x": true }])
        );
        assert_eq!(eval(&lua, "return 'x'").unwrap(), serde_json::json!("x"));
        assert_eq!(eval(&lua, "return 7").unwrap(), serde_json::json!(7));
    }

    #[test]
    fn huge_sparse_index_is_refused_without_allocating() {
        let lua = Lua::new();
        let err = eval(&lua, "return { [2^60] = 1 }").unwrap_err();
        assert!(err.to_string().contains("dense"), "{err}");
        let err = eval(&lua, "return { [1152921504606846976] = 1 }").unwrap_err();
        assert!(err.to_string().contains("dense"), "{err}");
    }

    #[test]
    fn holes_and_mixed_tables_are_refused() {
        let lua = Lua::new();
        assert!(eval(&lua, "return { [2] = 'b' }").is_err());
        assert!(eval(&lua, "return { 'a', name = 'x' }").is_err());
    }

    #[test]
    fn deep_and_cyclic_tables_are_refused() {
        let lua = Lua::new();
        let err = eval(
            &lua,
            "local t = {} local c = t for i = 1, 100 do c.n = {} c = c.n end return t",
        )
        .unwrap_err();
        assert!(err.to_string().contains("deeper"), "{err}");
        assert!(eval(&lua, "local t = {} t.self = t return t").is_err());
    }

    #[test]
    fn too_many_values_are_refused() {
        let lua = Lua::new();
        let err = eval(
            &lua,
            "local t = {} for i = 1, 200000 do t[i] = i end return t",
        )
        .unwrap_err();
        assert!(err.to_string().contains("more than"), "{err}");
    }

    #[test]
    fn functions_are_refused_and_nil_args_become_an_object() {
        let lua = Lua::new();
        assert!(eval(&lua, "return { f = function() end }").is_err());
        assert_eq!(args_to_json(&lua, None).unwrap(), serde_json::json!({}));
    }

    #[test]
    fn json_round_trips_into_lua() {
        let lua = Lua::new();
        let json = serde_json::json!({ "list": [1, 2], "ok": true });
        let value = from_json(&lua, &json).unwrap();
        assert_eq!(to_json(&lua, value).unwrap(), json);
    }
}
