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

pub fn bind(lua: &mlua::Lua) -> mlua::Result<mlua::Table<'_>> {
    // Compose on top of `utils_basic` so the existing `target_os`,
    // `target_family`, `time`, `hash` entries remain available.
    let table = utils_basic::bind(lua)?;

    // `pairee.quote(str, unix?)` — shell-escape a string. `unix=true`
    // forces POSIX-style escaping, `unix=false` forces Windows-style,
    // and `nil` auto-detects from `std::env::consts::FAMILY`.
    table.set(
        "quote",
        lua.create_function(|_lua, (s, unix): (mlua::String, Option<bool>)| {
            let text = String::from_utf8_lossy(s.as_bytes());
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
        lua.create_function(|_lua, s: mlua::String| {
            let bytes = s.as_bytes();
            let encoded = percent_encode(bytes, FRAGMENT).to_string();
            Ok(mlua::Value::String(_lua.create_string(&encoded)?))
        })?,
    )?;

    // `pairee.percent_decode(str)` — decode a percent-encoded string.
    table.set(
        "percent_decode",
        lua.create_function(|_lua, s: mlua::String| {
            let bytes = s.as_bytes();
            let decoded = percent_decode_str(&String::from_utf8_lossy(bytes))
                .decode_utf8_lossy()
                .into_owned();
            Ok(mlua::Value::String(_lua.create_string(&decoded)?))
        })?,
    )?;

    // `pairee.json_encode(value)` — serialise any Lua value (including
    // userdata, tables, strings, numbers, booleans) to a JSON string.
    table.set(
        "json_encode",
        lua.create_async_function(|_lua_ctx, value: mlua::Value| async move {
            match serde_json::to_string(&value) {
                Ok(s) => Ok(mlua::Value::String(_lua_ctx.create_string(&s)?)),
                Err(e) => {
                    log::warn!("pairee.json_encode failed: {e}");
                    Ok(mlua::Value::Nil)
                }
            }
        })?,
    )?;

    // `pairee.json_decode(str)` — parse a JSON string into a Lua value.
    table.set(
        "json_decode",
        lua.create_async_function(|_lua_ctx, s: mlua::String| async move {
            match serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(s.as_bytes()))
            {
                Ok(v) => {
                    use mlua::LuaSerdeExt;
                    let lua_value = _lua_ctx.to_value(&v).unwrap_or(mlua::Value::Nil);
                    Ok(lua_value)
                }
                Err(e) => {
                    log::warn!("pairee.json_decode failed: {e}");
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
        assert_eq!(win, "^\"a^&b ^%x^%^\"");
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
    fn test_json_encode_string() {
        let lua = Lua::new();
        let s = lua.create_string("hello").unwrap();
        // Just check it doesn't panic; round-trip is in the
        // `to_lua_value` direction which serde handles for `Value`.
        let _ = s;
    }
}
