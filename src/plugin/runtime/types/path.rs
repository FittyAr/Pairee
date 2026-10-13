//! `Path` userdata: a file system path with its parts and the usual path
//! operations, all done on the string (nothing touches the disk, so no
//! sandbox check is needed).
//!
//! ```lua
//! local p = pairee.Path("/home/me/notes.md")   -- or pairee.Path.os(...)
//! p.name, p.stem, p.ext, p.parent              -- "notes.md", "notes", "md", Path
//! p.is_absolute, p.has_root
//! p:join("x"), p:starts_with("/home"), p:ends_with("notes.md"), p:strip_prefix("/home")
//! tostring(p), p == other, "file: " .. p
//! ```
//!
//! The same operations back `Url` (see `url.rs`), which keeps the scheme and
//! host apart and reuses these helpers for its path part.

use mlua::{FromLua, Lua, MetaMethod, UserData, UserDataFields, UserDataMethods, Value};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LuaPath(pub PathBuf);

/// Final component (`None` for `/` or `..`-ended paths).
pub fn name_of(path: &Path) -> Option<String> {
    path.file_name().map(|n| n.to_string_lossy().into_owned())
}

pub fn stem_of(path: &Path) -> Option<String> {
    path.file_stem().map(|n| n.to_string_lossy().into_owned())
}

pub fn ext_of(path: &Path) -> Option<String> {
    path.extension().map(|n| n.to_string_lossy().into_owned())
}

/// Text of any value Lua code may give as a path: a string, a `Path`, a
/// `Url` (its path part) or a `File`.
pub fn path_arg(value: &Value) -> mlua::Result<PathBuf> {
    match value {
        Value::String(s) => Ok(PathBuf::from(s.to_string_lossy())),
        Value::UserData(ud) => {
            if let Ok(p) = ud.borrow::<LuaPath>() {
                return Ok(p.0.clone());
            }
            if let Ok(u) = ud.borrow::<super::url::LuaUrl>() {
                return Ok(u.path.clone());
            }
            if let Ok(f) = ud.borrow::<super::LuaFile>() {
                return Ok(PathBuf::from(&f.path));
            }
            Err(type_error(value))
        }
        other => Err(type_error(other)),
    }
}

fn type_error(value: &Value) -> mlua::Error {
    mlua::Error::RuntimeError(format!(
        "expected a path (string, Path, Url or File), got {}",
        value.type_name()
    ))
}

/// Text of a `..` operand: a string, number or any userdata with
/// `__tostring` that is one of the path types.
pub fn concat_operand(value: &Value) -> mlua::Result<String> {
    match value {
        Value::Integer(i) => Ok(i.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        Value::UserData(ud) if ud.borrow::<super::url::LuaUrl>().is_ok() => {
            Ok(ud.borrow::<super::url::LuaUrl>()?.to_string())
        }
        other => Ok(path_arg(other)?.to_string_lossy().into_owned()),
    }
}

impl FromLua for LuaPath {
    fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
        path_arg(&value).map(LuaPath)
    }
}

/// Adds the fields shared by `Path` and `Url` (computed from the path part
/// that `part` returns).
pub fn add_part_fields<T: 'static, F: UserDataFields<T>>(fields: &mut F, part: fn(&T) -> &Path) {
    fields.add_field_method_get("name", move |_, this| Ok(name_of(part(this))));
    fields.add_field_method_get("stem", move |_, this| Ok(stem_of(part(this))));
    fields.add_field_method_get("ext", move |_, this| Ok(ext_of(part(this))));
    fields.add_field_method_get("has_root", move |_, this| Ok(part(this).has_root()));
}

impl UserData for LuaPath {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        add_part_fields(fields, |this: &Self| this.0.as_path());
        fields.add_field_method_get("is_absolute", |_, this| Ok(this.0.is_absolute()));
        fields.add_field_method_get("parent", |_, this| {
            Ok(this.0.parent().map(|p| LuaPath(p.to_path_buf())))
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("join", |_, this, other: Value| {
            Ok(LuaPath(this.0.join(path_arg(&other)?)))
        });
        methods.add_method("starts_with", |_, this, base: Value| {
            Ok(this.0.starts_with(path_arg(&base)?))
        });
        methods.add_method("ends_with", |_, this, child: Value| {
            Ok(this.0.ends_with(path_arg(&child)?))
        });
        methods.add_method("strip_prefix", |_, this, base: Value| {
            let base = path_arg(&base)?;
            Ok(this
                .0
                .strip_prefix(base)
                .ok()
                .map(|p| LuaPath(p.to_path_buf())))
        });
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(this.0.to_string_lossy().into_owned())
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: Value| {
            Ok(path_arg(&other).is_ok_and(|p| p == this.0))
        });
        methods.add_meta_function(MetaMethod::Concat, |_, (a, b): (Value, Value)| {
            Ok(concat_operand(&a)? + &concat_operand(&b)?)
        });
    }
}

/// `pairee.Path`: callable (`Path(s)`) with `Path.os(s)`.
pub fn bind(lua: &Lua) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    table.set(
        "os",
        lua.create_function(|_, value: Value| path_arg(&value).map(LuaPath))?,
    )?;
    let meta = lua.create_table()?;
    meta.set(
        "__call",
        lua.create_function(|_, (_, value): (Value, Value)| path_arg(&value).map(LuaPath))?,
    )?;
    table.set_metatable(Some(meta))?;
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lua() -> Lua {
        let lua = Lua::new();
        lua.globals().set("Path", bind(&lua).unwrap()).unwrap();
        lua
    }

    #[test]
    fn parts_and_operations() {
        let lua = lua();
        let (name, stem, ext, parent_name): (String, String, String, String) = lua
            .load(
                r#"local p = Path("dir/sub/notes.tar.gz")
                   return p.name, p.stem, p.ext, p.parent.name"#,
            )
            .eval()
            .unwrap();
        assert_eq!(
            (
                name.as_str(),
                stem.as_str(),
                ext.as_str(),
                parent_name.as_str()
            ),
            ("notes.tar.gz", "notes.tar", "gz", "sub")
        );
        let (starts, ends, stripped, joined_name): (bool, bool, String, String) = lua
            .load(
                r#"local p = Path.os("dir/sub/a.txt")
                   return p:starts_with("dir"), p:ends_with(Path("a.txt")),
                          tostring(p:strip_prefix("dir")), p:join("x").name"#,
            )
            .eval()
            .unwrap();
        assert!(starts && ends);
        assert_eq!(PathBuf::from(stripped), PathBuf::from("sub/a.txt"));
        assert_eq!(joined_name, "x");
    }

    #[test]
    fn metamethods_and_missing_parts() {
        let lua = lua();
        let (same, text, no_ext, no_strip): (bool, String, Value, Value) = lua
            .load(
                r#"local p = Path("a/b")
                   return p == Path("a/b"), "<" .. p .. ">", p.ext, p:strip_prefix("zz")"#,
            )
            .eval()
            .unwrap();
        assert!(same);
        assert_eq!(text, format!("<{}>", Path::new("a/b").display()));
        assert!(no_ext.is_nil() && no_strip.is_nil());
        assert!(lua.load("return Path(42)").exec().is_err());
    }
}
