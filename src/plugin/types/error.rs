//! M2 typed userdata: `Error` — the standard error envelope for the
//! `(value, Error?)` multi-return convention described in
//! `docs/technical/plugin-roadmap.md` §4.3 (F3).
//!
//! Every async API in `fs.*`, `Command.*`, etc. returns either
//! `(value, nil)` on success or `(nil, Error)` on failure. The
//! `Error` carries a numeric `code`, a string `kind` (the OS
//! error family), and a human-readable message. A short Lua helper
//! `Err(s, ...)` is provided in `src/plugin/runtime/presets/ya.lua`
//! for plugin authors to construct ad-hoc errors.

use mlua::{MetaMethod, UserData, UserDataMethods};

/// The M2 `Error` userdata.
#[derive(Debug, Clone)]
pub struct Error {
    pub code: Option<i32>,
    pub kind: Option<String>,
    pub message: String,
}

impl Error {
    /// Build a custom error from a formatted message.
    pub fn custom(message: impl Into<String>) -> Self {
        Self {
            code: None,
            kind: Some("custom".to_string()),
            message: message.into(),
        }
    }

    /// Build a `fs`-flavoured error from an `std::io::Error`.
    pub fn from_io(err: &std::io::Error) -> Self {
        Self {
            code: Some(err.raw_os_error().unwrap_or(-1)),
            kind: Some(format!("{:?}", err.kind())),
            message: err.to_string(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

impl UserData for Error {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("code", |_lua, this, ()| Ok(this.code));
        methods.add_method("kind", |_lua, this, ()| Ok(this.kind.clone()));
        methods.add_method("message", |_lua, this, ()| Ok(this.message.clone()));

        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(this.message.clone())
        });
        methods.add_meta_method(MetaMethod::Concat, |_lua, this, other: mlua::Value| {
            let other_str = match other {
                mlua::Value::String(s) => s.to_str()?.to_string(),
                mlua::Value::Integer(i) => i.to_string(),
                mlua::Value::Number(n) => n.to_string(),
                mlua::Value::Boolean(b) => b.to_string(),
                mlua::Value::Nil => String::from("nil"),
                _ => String::from("?"),
            };
            Ok(format!("{}{}", this.message, other_str))
        });
    }
}

/// Build a Lua-callable constructor pair (`Error.custom`,
/// `Error.fs`) and register them on the given `pairee` table.
pub fn bind(lua: &mlua::Lua, pairee: &mlua::Table<'_>) -> mlua::Result<()> {
    let error_table = lua.create_table()?;

    // Error.custom(msg) — synchronous constructor.
    error_table.set(
        "custom",
        lua.create_function(|lua, msg: String| {
            let e = Error::custom(msg);
            lua.create_userdata(e).map(mlua::Value::UserData)
        })?,
    )?;

    // Error.fs({kind, code, message}) — constructor from a table.
    error_table.set(
        "fs",
        lua.create_function(|lua, opts: mlua::Table| {
            let kind: Option<String> = opts
                .get::<_, mlua::String>("kind")
                .ok()
                .and_then(|s| s.to_str().ok().map(|cow| cow.to_string()));
            let code: Option<i32> = opts.get::<_, i64>("code").ok().map(|n| n as i32);
            let message: String = opts
                .get::<_, mlua::String>("message")
                .ok()
                .and_then(|s| s.to_str().ok().map(|cow| cow.to_string()))
                .unwrap_or_default();
            let e = Error { code, kind, message };
            lua.create_userdata(e).map(mlua::Value::UserData)
        })?,
    )?;

    pairee.set("Error", error_table)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_error_custom_lua() {
        let lua = Lua::new();
        let err = Error::custom("bad input");
        lua.globals().set("e", lua.create_userdata(err).unwrap()).unwrap();
        let msg: String = lua.load("return e:message()").eval().unwrap();
        assert_eq!(msg, "bad input");
        let s: String = lua.load("return tostring(e)").eval().unwrap();
        assert_eq!(s, "bad input");
    }

    #[test]
    fn test_error_bind_constructors() {
        let lua = Lua::new();
        let pairee = lua.create_table().unwrap();
        bind(&lua, &pairee).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        let result: mlua::Value = lua
            .load(
                r#"
                local e = pairee.Error.custom("hello")
                return e:message()
                "#,
            )
            .eval()
            .unwrap();
        // The result is a string, not userdata, because we asked
        // for the message directly. Just assert it round-trips.
        let s = match result {
            mlua::Value::String(s) => s.to_str().unwrap().to_string(),
            other => panic!("expected string, got {:?}", other),
        };
        assert_eq!(s, "hello");
    }
}
