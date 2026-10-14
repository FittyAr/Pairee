//! Extra `pairee.fs` operations: mkdir, remove, rename, copy, read_dir,
//! file and cha.

use super::path::{Access, FsPolicy, lua_to_path};
use super::target::RemoveKind;
use crate::plugin::runtime::types::{LuaCha, LuaError, LuaFile};
use mlua::{Lua, Table, Value};
use std::sync::Arc;

pub fn bind_extra(lua: &Lua, fs: &Table, policy: &Arc<FsPolicy>) -> mlua::Result<()> {
    let p = Arc::clone(policy);
    fs.set(
        "mkdir",
        lua.create_function(move |_, (kind, url): (String, Value)| {
            let path = lua_to_path(&p, url, Access::Write)?;
            path.create_dir(kind == "dir_all")
                .map_err(|e| mlua::Error::RuntimeError(format!("mkdir failed: {e}")))
        })?,
    )?;

    let p = Arc::clone(policy);
    fs.set(
        "remove",
        lua.create_function(move |_, (kind, url): (String, Value)| {
            let path = lua_to_path(&p, url, Access::Write)?;
            path.remove(RemoveKind::from_lua_name(&kind))
                .map_err(|e| mlua::Error::RuntimeError(format!("remove failed: {e}")))
        })?,
    )?;

    let p = Arc::clone(policy);
    fs.set(
        "rename",
        lua.create_function(move |_, (from, to): (Value, Value)| {
            let from = lua_to_path(&p, from, Access::Write)?;
            let to = lua_to_path(&p, to, Access::Write)?;
            from.rename_to(&to)
                .map_err(|e| mlua::Error::RuntimeError(format!("rename failed: {e}")))
        })?,
    )?;

    let p = Arc::clone(policy);
    fs.set(
        "copy",
        lua.create_function(move |_, (from, to): (Value, Value)| {
            let from = lua_to_path(&p, from, Access::Read)?;
            let to = lua_to_path(&p, to, Access::Write)?;
            from.copy_to(&to)
                .map_err(|e| mlua::Error::RuntimeError(format!("copy failed: {e}")))
        })?,
    )?;

    let p = Arc::clone(policy);
    fs.set(
        "read_dir",
        lua.create_function(move |_, url: Value| {
            let path = lua_to_path(&p, url, Access::Read)?;
            Ok(path
                .list()
                .iter()
                .map(|p| LuaFile::from_path(p))
                .collect::<Vec<_>>())
        })?,
    )?;

    let p = Arc::clone(policy);
    fs.set(
        "file",
        lua.create_function(move |_, url: Value| {
            let path = lua_to_path(&p, url, Access::Read)?;
            Ok(LuaFile::from_path(path.path()))
        })?,
    )?;

    // `cha(path, follow?)` -> Cha, nil | nil, Error (a sandbox refusal is a
    // "PermissionDenied" Error, not a thrown error).
    let p = Arc::clone(policy);
    fs.set(
        "cha",
        lua.create_function(move |lua, (url, follow): (Value, Option<bool>)| {
            let result = lua_to_path(&p, url, Access::Read)
                .map_err(|e| LuaError::denied(e.to_string()))
                .and_then(|target| {
                    LuaCha::read(target.path(), follow.unwrap_or(false)).map_err(LuaError::from)
                });
            LuaError::pair(lua, result)
        })?,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::manager::PluginRequest;
    use crate::plugin::runtime::bindings::fs::bind;
    use tokio::sync::mpsc;

    fn bind_fs(lua: &Lua) -> Table {
        let (tx, _rx) = mpsc::channel::<PluginRequest>(1);
        bind(
            lua,
            std::sync::Arc::new(FsPolicy::new(std::path::Path::new("demo"), true, false)),
            std::sync::Arc::new(crate::plugin::command_policy::CommandPolicy::Unrestricted),
            tx,
        )
        .unwrap()
    }

    #[test]
    fn mkdir_write_read_copy_rename_remove() {
        let lua = Lua::new();
        let fs = bind_fs(&lua);
        lua.globals().set("fs", fs).unwrap();

        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("nested").join("leaf");
        let dir_s = dir.to_string_lossy().to_string();
        lua.globals().set("dir", dir_s.as_str()).unwrap();

        lua.load(r#"fs.mkdir("dir_all", dir)"#).exec().unwrap();
        assert!(dir.is_dir());

        let file = dir.join("a.txt");
        let file_s = file.to_string_lossy().to_string();
        lua.globals().set("file", file_s.as_str()).unwrap();
        lua.load(r#"fs.write(file, "hello")"#).exec().unwrap();
        let read: String = lua.load(r#"return fs.read(file)"#).eval().unwrap();
        assert_eq!(read, "hello");

        let copy = dir.join("b.txt");
        let copy_s = copy.to_string_lossy().to_string();
        lua.globals().set("copy", copy_s.as_str()).unwrap();
        let n: u64 = lua.load(r#"return fs.copy(file, copy)"#).eval().unwrap();
        assert_eq!(n, 5);
        assert_eq!(std::fs::read_to_string(&copy).unwrap(), "hello");

        let renamed = dir.join("c.txt");
        let renamed_s = renamed.to_string_lossy().to_string();
        lua.globals().set("renamed", renamed_s.as_str()).unwrap();
        lua.load(r#"fs.rename(copy, renamed)"#).exec().unwrap();
        assert!(renamed.exists());
        assert!(!copy.exists());

        let listed: Vec<LuaFile> = lua.load(r#"return fs.read_dir(dir)"#).eval().unwrap();
        assert_eq!(listed.len(), 2);

        lua.load(r#"fs.remove("file", file)"#).exec().unwrap();
        assert!(!file.exists());
        lua.load(r#"fs.remove("dir_all", dir)"#).exec().unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn file_constructor_returns_userdata() {
        let lua = Lua::new();
        let fs = bind_fs(&lua);
        lua.globals().set("fs", fs).unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let p = tmp.path().to_string_lossy().to_string();
        lua.globals().set("p", p.as_str()).unwrap();
        let name: String = lua.load(r#"return fs.file(p).name"#).eval().unwrap();
        assert_eq!(
            name,
            tmp.path().file_name().unwrap().to_string_lossy().as_ref()
        );
    }

    #[test]
    fn cha_returns_a_value_or_an_error() {
        let lua = Lua::new();
        lua.globals().set("fs", bind_fs(&lua)).unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "abc").unwrap();
        lua.globals()
            .set("dir", dir.path().to_string_lossy().as_ref())
            .unwrap();
        let (len, is_dir, kind): (u64, bool, String) = lua
            .load(
                r#"local c = fs.cha(dir .. "/a.txt")
                   local _, err = fs.cha(dir .. "/missing")
                   return c.len, fs.cha(dir).is_dir, err.kind"#,
            )
            .eval()
            .unwrap();
        assert_eq!((len, is_dir, kind.as_str()), (3, true, "NotFound"));

        let (tx, _rx) = mpsc::channel::<PluginRequest>(1);
        let jailed = bind(
            &lua,
            std::sync::Arc::new(FsPolicy::new(std::path::Path::new("demo"), false, false)),
            std::sync::Arc::new(crate::plugin::command_policy::CommandPolicy::Unrestricted),
            tx,
        )
        .unwrap();
        lua.globals().set("jailed", jailed).unwrap();
        let (value, kind): (Value, String) = lua
            .load(r#"local v, e = jailed.cha(dir .. "/a.txt") return v, e.kind"#)
            .eval()
            .unwrap();
        assert!(value.is_nil());
        assert_eq!(kind, "PermissionDenied");
    }
}
