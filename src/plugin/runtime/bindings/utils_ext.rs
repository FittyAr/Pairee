//! Extended utility bindings for the plugin runtime (M1).
//!
//! Exposes the cross-platform `pairee.utils.{sleep, time, hash,
//! target_os, target_family}` set from `utils_basic.rs` together with the
//! document-oriented helpers added in M1:
//!
//! - `pairee.quote(str, unix?)` — shell-escape a string.
//! - `pairee.percent_encode(str)` / `pairee.percent_decode(str)` —
//!   percent-encode / percent-decode a string (RFC 3986).
//! - `pairee.json_encode(value)` / `pairee.json_decode(str)` — JSON
//!   serialise / parse a Lua value.
//! - `pairee.sleep(secs)` — async sleep (returns a Future that resolves
//!   after the given number of seconds).
//!
//! All helpers here are pure (no state side effects) and safe under
//! Secure Mode. The function `sleep` is the only async one; it uses
//! `tokio::time::sleep` which yields to the runtime and does not block
//! any worker thread.

use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, percent_encode};

use super::utils_basic;
use crate::plugin::runtime::lua_json;

pub fn bind(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
    // Compose on top of `utils_basic` so the existing `target_os`,
    // `target_family`, `time`, `hash` entries remain available.
    let table = utils_basic::bind(lua)?;
    super::identity::bind_into(lua, &table)?;

    // `pairee.quote(str, unix?)` — shell-escape a string. `unix=true`
    // forces POSIX-style escaping, `unix=false` forces Windows-style,
    // and `nil` auto-detects from `std::env::consts::FAMILY`.
    table.set(
        "quote",
        lua.create_function(|_lua, (s, unix): (mlua::LuaString, Option<bool>)| {
            let bytes = s.as_bytes();
            let text = String::from_utf8_lossy(&bytes);
            let escaped = match unix {
                Some(true) => crate::shell::quote_posix(&text),
                Some(false) => crate::shell::quote_cmd(&text),
                None => crate::shell::quote_native(&text),
            };
            Ok(mlua::Value::String(_lua.create_string(&escaped)?))
        })?,
    )?;

    // `pairee.percent_encode(str)` — encode a string per RFC 3986
    // (unreserved characters preserved).
    table.set(
        "percent_encode",
        lua.create_function(|_lua, s: mlua::LuaString| {
            let bytes = &s.as_bytes();
            let encoded = percent_encode(bytes, FRAGMENT).to_string();
            Ok(mlua::Value::String(_lua.create_string(&encoded)?))
        })?,
    )?;

    // `pairee.percent_decode(str)` — decode a percent-encoded string.
    table.set(
        "percent_decode",
        lua.create_function(|_lua, s: mlua::LuaString| {
            let bytes = &s.as_bytes();
            let decoded = percent_decode_str(&String::from_utf8_lossy(bytes))
                .decode_utf8_lossy()
                .into_owned();
            Ok(mlua::Value::String(_lua.create_string(&decoded)?))
        })?,
    )?;

    // `pairee.json_encode(value)` — serialise a Lua value (tables, strings,
    // numbers, booleans) to a JSON string with the shared bounded
    // converter; values it refuses return nil.
    table.set(
        "json_encode",
        lua.create_function(|lua_ctx, value: mlua::Value| {
            let encoded = lua_json::to_json(lua_ctx, value)
                .and_then(|json| serde_json::to_string(&json).map_err(mlua::Error::external));
            match encoded {
                Ok(s) => Ok(mlua::Value::String(lua_ctx.create_string(&s)?)),
                Err(e) => {
                    tracing::warn!("pairee.json_encode failed: {e}");
                    Ok(mlua::Value::Nil)
                }
            }
        })?,
    )?;

    // `pairee.json_decode(str)` — parse a JSON string into a Lua value.
    table.set(
        "json_decode",
        lua.create_function(|lua_ctx, s: mlua::LuaString| {
            let bytes = s.as_bytes();
            let text = String::from_utf8_lossy(&bytes);
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) => Ok(lua_json::from_json(lua_ctx, &v).unwrap_or(mlua::Value::Nil)),
                Err(e) => {
                    tracing::warn!("pairee.json_decode failed: {e}");
                    Ok(mlua::Value::Nil)
                }
            }
        })?,
    )?;

    // `pairee.sleep(secs)` — async sleep. Returns a Future that resolves
    // after the given number of seconds. Negative durations are rejected
    // to keep the behaviour predictable.
    table.set(
        "sleep",
        lua.create_async_function(|_lua_ctx, secs: f64| async move {
            if secs < 0.0 {
                log::warn!("pairee.sleep received a negative duration ({secs}); ignoring");
                return Ok(mlua::Value::Nil);
            }
            tokio::time::sleep(std::time::Duration::from_secs_f64(secs)).await;
            Ok(mlua::Value::Nil)
        })?,
    )?;

    Ok(table)
}

/// Unreserved characters per RFC 3986 §2.3. We also leave the slash
/// alone to match the common "fragment" set used in URLs.
const FRAGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'!')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn quote_uses_shared_shell_quoting() {
        let lua = Lua::new();
        let utils = bind(&lua).unwrap();
        let quote: mlua::Function = utils.get("quote").unwrap();
        let unix: String = quote.call(("it's", true)).unwrap();
        assert_eq!(unix, "'it'\\''s'");
        let win: String = quote.call(("a&b %x%", false)).unwrap();
        assert_eq!(win, "^\"a^&b ^%^x^%^\"");
    }

    #[test]
    fn test_percent_encode_decode_roundtrip() {
        let original = "hello world ?#&=+";
        let encoded = percent_encode(original.as_bytes(), FRAGMENT).to_string();
        assert!(!encoded.contains(' '));
        let decoded = percent_decode_str(&encoded)
            .decode_utf8_lossy()
            .into_owned();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_percent_encode_preserves_unreserved() {
        // RFC 3986 unreserved characters must NOT be percent-encoded.
        let s = "abcXYZ0129-._~";
        let encoded = percent_encode(s.as_bytes(), FRAGMENT).to_string();
        assert_eq!(encoded, s);
    }

    #[test]
    fn json_encode_and_decode_use_the_bounded_converter() {
        let lua = Lua::new();
        lua.globals().set("u", bind(&lua).unwrap()).unwrap();
        let (encoded, sparse, name): (String, mlua::Value, String) = lua
            .load(
                r#"return u.json_encode({ a = { 1, 2 } }), u.json_encode({ [2^40] = 1 }),
                   u.json_decode('{"name":"x"}').name"#,
            )
            .eval()
            .unwrap();
        assert_eq!(encoded, r#"{"a":[1,2]}"#);
        assert!(sparse.is_nil());
        assert_eq!(name, "x");
    }
}
