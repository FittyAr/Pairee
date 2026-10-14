//! `Error` userdata: the error half of the `(value, Error?)` convention
//! that the newer `pairee.*` APIs follow (`pairee.clipboard`,
//! `pairee.fs.cha`, `pairee.image.*`, `pairee.state.*`).
//!
//! On success such a function returns its value and `nil`; on failure it
//! returns `nil` (or `false`) and an `Error` with `kind` (an
//! `std::io::ErrorKind` name such as `"NotFound"`, or `nil`), `code` (the OS
//! error number, or `nil`) and `message`. `tostring(err)` is the message and
//! `..` concatenates it with strings. Older APIs keep throwing.

use mlua::{IntoLuaMulti, Lua, MetaMethod, MultiValue, UserData, UserDataFields, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LuaError {
    pub kind: Option<String>,
    pub code: Option<i32>,
    pub message: String,
}

impl LuaError {
    /// An error with only a message (`Error.custom`).
    pub fn custom(message: impl Into<String>) -> Self {
        Self {
            kind: None,
            code: None,
            message: message.into(),
        }
    }

    /// A `PermissionDenied` error (a sandbox refusal).
    pub fn denied(message: impl Into<String>) -> Self {
        Self {
            kind: Some("PermissionDenied".into()),
            code: None,
            message: message.into(),
        }
    }

    /// The kind, OS code and text of an I/O error.
    pub fn from_io(err: &std::io::Error) -> Self {
        Self {
            kind: Some(format!("{:?}", err.kind())),
            code: err.raw_os_error(),
            message: err.to_string(),
        }
    }

    /// `(value, nil)` on success, `(nil, Error)` on failure.
    pub fn pair<T: mlua::IntoLua>(lua: &Lua, result: Result<T, Self>) -> mlua::Result<MultiValue> {
        match result {
            Ok(value) => (value, Value::Nil).into_lua_multi(lua),
            Err(err) => (Value::Nil, err).into_lua_multi(lua),
        }
    }

    /// `ok, nil` or `false, err`: the result of an operation that reports
    /// success as a boolean.
    pub fn status(lua: &Lua, result: Result<bool, Self>) -> mlua::Result<MultiValue> {
        match result {
            Ok(ok) => (ok, Value::Nil).into_lua_multi(lua),
            Err(err) => (false, err).into_lua_multi(lua),
        }
    }
}

impl From<std::io::Error> for LuaError {
    fn from(err: std::io::Error) -> Self {
        Self::from_io(&err)
    }
}

/// Text of one operand of `..` with an `Error` on either side.
fn concat_operand(value: &Value) -> mlua::Result<String> {
    match value {
        Value::String(s) => Ok(s.to_string_lossy()),
        Value::Integer(i) => Ok(i.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        Value::UserData(ud) => Ok(ud.borrow::<LuaError>()?.message.clone()),
        other => Err(mlua::Error::RuntimeError(format!(
            "attempt to concatenate an Error with a {} value",
            other.type_name()
        ))),
    }
}

impl UserData for LuaError {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("kind", |_, this| Ok(this.kind.clone()));
        fields.add_field_method_get("code", |_, this| Ok(this.code));
        fields.add_field_method_get("message", |_, this| Ok(this.message.clone()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(this.message.clone()));
        methods.add_meta_function(MetaMethod::Concat, |_, (a, b): (Value, Value)| {
            Ok(concat_operand(&a)? + &concat_operand(&b)?)
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: mlua::AnyUserData| {
            Ok(other.borrow::<LuaError>().is_ok_and(|o| *o == *this))
        });
    }
}

/// `pairee.Error`: `Error.custom(msg)` and `Error.fs{kind, code, message}`.
pub fn bind(lua: &Lua) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    table.set(
        "custom",
        lua.create_function(|_, message: String| Ok(LuaError::custom(message)))?,
    )?;
    table.set(
        "fs",
        lua.create_function(|_, spec: mlua::Table| {
            Ok(LuaError {
                kind: spec.get("kind")?,
                code: spec.get("code")?,
                message: spec.get::<Option<String>>("message")?.unwrap_or_default(),
            })
        })?,
    )?;
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_show_their_fields_and_concatenate() {
        let lua = Lua::new();
        lua.globals().set("Error", bind(&lua).unwrap()).unwrap();
        let (text, joined, kind, code, same): (String, String, String, i32, bool) = lua
            .load(
                r#"
                local e = Error.fs({ kind = "NotFound", code = 2, message = "gone" })
                return tostring(e), "x: " .. e .. "!", e.kind, e.code,
                       e == Error.fs({ kind = "NotFound", code = 2, message = "gone" })
                "#,
            )
            .eval()
            .unwrap();
        assert_eq!((text.as_str(), joined.as_str()), ("gone", "x: gone!"));
        assert_eq!((kind.as_str(), code), ("NotFound", 2));
        assert!(same);
        let custom: (Value, Value) = lua
            .load("local e = Error.custom('boom') return e.kind, e.code")
            .eval()
            .unwrap();
        assert!(custom.0.is_nil() && custom.1.is_nil());
    }

    #[test]
    fn io_errors_keep_kind_and_code() {
        let err = std::fs::read("definitely/not/here").unwrap_err();
        let e = LuaError::from_io(&err);
        assert_eq!(e.kind.as_deref(), Some("NotFound"));
        assert!(e.code.is_some());
    }

    #[test]
    fn pair_returns_value_or_error() {
        let lua = Lua::new();
        let ok = LuaError::pair(&lua, Ok::<_, LuaError>(5)).unwrap();
        assert_eq!(ok.len(), 2);
        assert!(matches!(ok[0], Value::Integer(5)) && ok[1].is_nil());
        let bad = LuaError::pair::<i32>(&lua, Err(LuaError::custom("no"))).unwrap();
        assert!(bad[0].is_nil() && matches!(bad[1], Value::UserData(_)));
    }
}
