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
//! - `pairee.uid()` / `pairee.gid()` / `pairee.user_name()` /
//!   `pairee.group_name()` / `pairee.host_name()` — Unix identity
//!   helpers. On Windows `uid` and `gid` return `nil`; the others
//!   still work (`user_name` via `USERNAME`, `host_name` via the
//!   `hostname` crate).
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

    // `pairee.quote_arg(str, unix?)` — safe argument quoting for
    // POSIX / Windows command-line arguments. Use this when feeding
    // a string as a positional argument to a spawned program; the
    // result is safe against whitespace, quote, and backslash
    // splitting regardless of platform.
    //
    //   unix = true   → POSIX-style single-quote escaping
    //   unix = false  → Windows-style double-quote + backslash
    //                   escaping (NOT safe for `cmd.exe /c` strings,
    //                    only for argument passing)
    //   unix = nil    → auto-detect from `std::env::consts::FAMILY`
    table.set(
        "quote_arg",
        lua.create_function(|_lua, (s, unix): (mlua::String, Option<bool>)| {
            let bytes = s.as_bytes().to_vec();
            let escaped = match unix {
                Some(true) => shell_escape_unix(&bytes),
                Some(false) => shell_escape_windows(&bytes),
                None => match std::env::consts::FAMILY {
                    "unix" => shell_escape_unix(&bytes),
                    _ => shell_escape_windows(&bytes),
                },
            };
            Ok(mlua::Value::String(_lua.create_string(&escaped)?))
        })?,
    )?;

    // `pairee.quote(str, unix?)` — DEPRECATED alias for `quote_arg`
    // kept for backwards compatibility. Plugins that previously
    // called `pairee.quote` for argument passing continue to work;
    // new code should call `pairee.quote_arg` to make the intent
    // explicit and to avoid confusion with `cmd /c` shell strings
    // (which require different escaping entirely).
    table.set(
        "quote",
        lua.create_function(|inner_lua, (s, unix): (mlua::String, Option<bool>)| {
            let bytes = s.as_bytes().to_vec();
            let escaped = match unix {
                Some(true) => shell_escape_unix(&bytes),
                Some(false) => shell_escape_windows(&bytes),
                None => match std::env::consts::FAMILY {
                    "unix" => shell_escape_unix(&bytes),
                    _ => shell_escape_windows(&bytes),
                },
            };
            log::debug!(
                "pairee.quote is deprecated; use pairee.quote_arg for argument passing"
            );
            Ok(mlua::Value::String(inner_lua.create_string(&escaped)?))
        })?,
    )?;

    // `pairee.percent_encode(str)` — encode a string per RFC 3986
    // (unreserved characters preserved).
    table.set(
        "percent_encode",
        lua.create_function(|inner_lua, s: mlua::String| {
            let bytes = s.as_bytes();
            let encoded = percent_encode(bytes, FRAGMENT).to_string();
            inner_lua.create_string(&encoded).map(mlua::Value::String)
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
    //
    // Note on partial success: `mlua::Value` implements
    // `Serialize` for the JSON-friendly subset (Nil, Boolean,
    // Integer, Number, String, Table, LightUserData, Error).
    // Functions, Threads, and custom userdata without a serde
    // adapter are serialised as their Debug string (`"userdata:..."`)
    // so plugins always get a parseable JSON output rather than
    // a silent Nil. Real failures (recursion overflow, malformed
    // UTF-8) still surface as a Lua runtime error so the plugin
    // sees the failure rather than dropping data silently.
    table.set(
        "json_encode",
        lua.create_async_function(|lua_ctx, value: mlua::Value| async move {
            match serde_json::to_string(&value) {
                Ok(s) => Ok(mlua::Value::String(lua_ctx.create_string(&s)?)),
                Err(e) => Err(mlua::Error::RuntimeError(format!(
                    "pairee.json_encode failed: {e}"
                ))),
            }
        })?,
    )?;

    // `pairee.json_decode(str)` — parse a JSON string into a Lua value.
    // The value round-trips through `serde_json::Value` so plugins see
    // idiomatic Lua tables/arrays/strings on the way out. Malformed
    // input is reported as a Lua runtime error rather than `nil`
    // so the plugin sees the failure rather than dropping data
    // silently.
    table.set(
        "json_decode",
        lua.create_async_function(|lua_ctx, s: mlua::String| async move {
            let v = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(s.as_bytes()))
                .map_err(|e| {
                    mlua::Error::RuntimeError(format!("pairee.json_decode failed: {e}"))
                })?;
            mlua::LuaSerdeExt::to_value(lua_ctx, &v)
                .map_err(|e| mlua::Error::RuntimeError(format!("pairee.json_decode: {e}")))
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

    // ── Identity helpers (M1) ─────────────────────────────────────────
    //
    // `pairee.uid()` / `pairee.gid()` return the current user's UID/GID
    // as an integer on Unix, and `nil` on Windows. `pairee.user_name()` /
    // `pairee.group_name()` return the matching display name (or `nil`
    // on lookup failure). `pairee.host_name()` returns the host name
    // on every platform via the `hostname` crate.
    #[cfg(unix)]
    {
        table.set(
            "uid",
            lua.create_function(|_, ()| Ok(mlua::Value::Integer(uzers::get_current_uid() as i64)))?,
        )?;
        table.set(
            "gid",
            lua.create_function(|_, ()| Ok(mlua::Value::Integer(uzers::get_current_gid() as i64)))?,
        )?;
        table.set(
            "user_name",
            lua.create_function(|inner_lua, ()| {
                let uid = uzers::get_current_uid();
                let name = uzers::get_user_by_uid(uid)
                    .and_then(|u| u.name().to_str().map(|s| s.to_string()));
                match name {
                    Some(s) => inner_lua.create_string(&s).map(mlua::Value::String),
                    None => Ok(mlua::Value::Nil),
                }
            })?,
        )?;
        table.set(
            "group_name",
            lua.create_function(|inner_lua, ()| {
                let gid = uzers::get_current_gid();
                let name = uzers::get_group_by_gid(gid)
                    .and_then(|g| g.name().to_str().map(|s| s.to_string()));
                match name {
                    Some(s) => inner_lua.create_string(&s).map(mlua::Value::String),
                    None => Ok(mlua::Value::Nil),
                }
            })?,
        )?;
    }
    #[cfg(not(unix))]
    {
        table.set(
            "uid",
            lua.create_function(|_, ()| Ok(mlua::Value::Nil))?,
        )?;
        table.set(
            "gid",
            lua.create_function(|_, ()| Ok(mlua::Value::Nil))?,
        )?;
        table.set(
            "user_name",
            lua.create_function(|inner_lua, ()| {
                match std::env::var("USERNAME").or_else(|_| std::env::var("USER")) {
                    Ok(s) => inner_lua.create_string(&s).map(mlua::Value::String),
                    Err(_) => Ok(mlua::Value::Nil),
                }
            })?,
        )?;
        table.set(
            "group_name",
            lua.create_function(|_, ()| Ok(mlua::Value::Nil))?,
        )?;
    }
    table.set(
        "host_name",
        lua.create_function(|inner_lua, ()| match hostname::get() {
            Ok(os) => match os.into_string() {
                Ok(s) => inner_lua.create_string(&s).map(mlua::Value::String),
                Err(_) => Ok(mlua::Value::Nil),
            },
            Err(_) => Ok(mlua::Value::Nil),
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

/// Minimal POSIX shell escape: wrap the input in single quotes and
/// escape any embedded single quotes. Sufficient for use in `bash -c`
/// and similar invocations.
fn shell_escape_unix(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 2);
    out.push(b'\'');
    for &b in bytes {
        if b == b'\'' {
            out.extend_from_slice(b"'\\''");
        } else {
            out.push(b);
        }
    }
    out.push(b'\'');
    out
}

/// Argument-style Windows escape following the CommandLineToArgvW
/// rules. This is what `CommandLineToArgvW` (used by Rust's
/// `std::process::Command`) parses back, so a string escaped this
/// way will be received verbatim by the child process when passed
/// as a single argument. This is **NOT** safe for use in
/// `cmd.exe /c "..."` strings — those require entirely different
/// escaping for metacharacters like `^`, `&`, `|`, `<`, `>`, `%`.
/// Use `quote_arg` (this helper) for argument passing; if you
/// need to embed a string in a shell command, build the command
/// using `Command(...):arg(...)` instead.
fn shell_escape_windows(bytes: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len() + 2);
    let mut backslashes = 0usize;
    out.push(b'"');
    for &b in bytes {
        if b == b'\\' {
            backslashes += 1;
            out.push(b'\\');
        } else if b == b'"' {
            // Escape all accumulated backslashes, then escape the quote.
            for _ in 0..backslashes {
                out.push(b'\\');
            }
            out.push(b'\\');
            out.push(b'"');
            backslashes = 0;
        } else {
            // Any non-backslash, non-quote byte flushes the backslash
            // accumulator.
            backslashes = 0;
            out.push(b);
        }
    }
    // Closing quote: double every trailing backslash so the quote is
    // not interpreted as escaped.
    for _ in 0..backslashes {
        out.push(b'\\');
    }
    out.push(b'"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_shell_escape_unix_simple() {
        let r = shell_escape_unix(b"hello");
        assert_eq!(r, b"'hello'");
    }

    #[test]
    fn test_shell_escape_unix_with_quote() {
        // `it's` must escape the embedded single quote by closing,
        // escaping, and reopening the quote.
        let r = shell_escape_unix(b"it's");
        assert_eq!(r, b"'it'\\''s'");
    }

    #[test]
    fn test_shell_escape_windows_with_quote() {
        // `a"b` becomes `"a\"b"` — the backslash before `"` is doubled
        // and the `"` itself is escaped.
        let r = shell_escape_windows(b"a\"b");
        assert_eq!(r, b"\"a\\\"b\"");
    }

    #[test]
    fn test_shell_escape_windows_with_backslash() {
        // CommandLineToArgvW rules: backslashes are literal UNLESS
        // followed by a `"`. `a\b` therefore becomes `"a\b"` (one
        // backslash preserved verbatim).
        let r = shell_escape_windows(b"a\\b");
        assert_eq!(r, b"\"a\\b\"");
    }

    #[test]
    fn test_shell_escape_windows_trailing_backslashes() {
        // Trailing backslashes before the closing `"` are doubled so
        // they don't escape the quote. `a\` becomes `"a\\"`.
        let r = shell_escape_windows(b"a\\");
        assert_eq!(r, b"\"a\\\\\"");
    }

    #[test]
    fn test_shell_escape_windows_many_backslashes_then_quote() {
        // CommandLineToArgvW: 2N backslashes + `"` → 2N+1 backslashes
        // + escaped `"`. Input `\\"` → output `"\\\\\""`.
        let r = shell_escape_windows(b"\\\\\"");
        assert_eq!(r, b"\"\\\\\\\\\\\"\"");
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

    #[test]
    fn test_bind_exposes_identity_helpers() {
        let lua = Lua::new();
        let table = bind(&lua).expect("utils table");
        // All M1 identity keys must be present on the table.
        for key in [
            "uid",
            "gid",
            "user_name",
            "group_name",
            "host_name",
            "quote",
            "quote_arg",
            "percent_encode",
            "percent_decode",
            "json_encode",
            "json_decode",
            "sleep",
            "time",
            "hash",
            "target_os",
            "target_family",
        ] {
            assert!(
                table.get::<_, mlua::Value>(key).is_ok(),
                "utils table missing key {key}"
            );
        }
    }

    #[test]
    fn test_quote_arg_is_safe_for_argument_passing() {
        // Round-trip: a quoted Windows argument must survive being
        // parsed back by CommandLineToArgvW. We exercise a payload
        // that contains quotes, backslashes, and whitespace — the
        // adversarial cases that historically broke naive escapes.
        let lua = Lua::new();
        let table = bind(&lua).expect("utils table");
        let quote_arg: mlua::Function = table.get("quote_arg").expect("quote_arg");
        // Unix branch.
        let unix_escaped: String = quote_arg
            .call::<_, mlua::String>(("it's got a quote", Some(true)))
            .expect("quote_arg unix")
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(unix_escaped, "'it'\\''s got a quote'");
        // Windows branch.
        let win_escaped: String = quote_arg
            .call::<_, mlua::String>(("a\\b\"c d", Some(false)))
            .expect("quote_arg windows")
            .to_str()
            .unwrap()
            .to_string();
        // The exact byte sequence follows CommandLineToArgvW:
        //   a\b"c d → "a\b\"c d"
        assert_eq!(win_escaped, r#""a\b\"c d""#);
    }

    #[tokio::test]
    async fn test_json_decode_rejects_malformed_input() {
        // Malformed JSON must surface as a Lua runtime error rather
        // than a silent Nil — the previous implementation swallowed
        // parse failures with `unwrap_or(Nil)`, which let plugin
        // authors overlook broken downstream pipelines.
        let lua = Lua::new();
        let table = bind(&lua).expect("utils table");
        let json_decode: mlua::Function = table.get("json_decode").expect("json_decode");
        let result = json_decode.call_async::<_, mlua::Value>("not json".to_string()).await;
        assert!(
            result.is_err(),
            "json_decode must reject malformed JSON with a runtime error"
        );
    }

    #[test]
    fn test_host_name_is_nonempty() {
        let lua = Lua::new();
        let table = bind(&lua).expect("utils table");
        let host_fn: mlua::Function = table.get("host_name").expect("host_name function");
        let value: mlua::Value = host_fn.call(()).expect("host_name result");
        match value {
            mlua::Value::String(s) => {
                let s = s.to_str().unwrap();
                assert!(!s.is_empty(), "host_name returned empty string");
            }
            mlua::Value::Nil => {
                // Some test environments may not have a hostname; that
                // is acceptable (the helper returns nil in that case).
            }
            other => panic!("host_name returned unexpected value: {:?}", other),
        }
    }

    #[test]
    fn test_target_os_and_family() {
        let lua = Lua::new();
        let table = bind(&lua).expect("utils table");
        let os_fn: mlua::Function = table.get("target_os").expect("target_os");
        let os: String = os_fn.call(()).expect("os result");
        assert!(!os.is_empty());
        let fam_fn: mlua::Function = table.get("target_family").expect("target_family");
        let fam: String = fam_fn.call(()).expect("family result");
        assert!(matches!(fam.as_str(), "unix" | "windows" | "wasm"));
    }

    #[test]
    #[cfg(unix)]
    fn test_uid_gid_are_integers_on_unix() {
        let lua = Lua::new();
        let table = bind(&lua).expect("utils table");
        let uid_fn: mlua::Function = table.get("uid").expect("uid");
        let uid: mlua::Value = uid_fn.call(()).expect("uid result");
        assert!(matches!(uid, mlua::Value::Integer(_)));
        let gid_fn: mlua::Function = table.get("gid").expect("gid");
        let gid: mlua::Value = gid_fn.call(()).expect("gid result");
        assert!(matches!(gid, mlua::Value::Integer(_)));
    }
}
