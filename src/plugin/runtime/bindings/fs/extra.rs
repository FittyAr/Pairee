//! Extra `pairee.fs` operations: mkdir, remove, rename, copy, read_dir, file.

use super::path::{
    fs_copy, fs_create_dir, fs_create_dir_all, fs_remove_dir, fs_remove_dir_all, fs_remove_file,
    fs_rename, lua_to_path,
};
use crate::plugin::runtime::types::LuaFile;
use mlua::{Lua, Table, Value};

pub fn bind_extra(lua: &Lua, fs: &Table<'_>) -> mlua::Result<()> {
    fs.set(
        "mkdir",
        lua.create_function(|lua_ctx, (kind, url): (String, Value)| {
            let path = lua_to_path(lua_ctx, url)?;
            let result = match kind.as_str() {
                "dir_all" => fs_create_dir_all(&path),
                _ => fs_create_dir(&path),
            };
            result.map_err(|e| mlua::Error::RuntimeError(format!("mkdir failed: {e}")))
        })?,
    )?;

    fs.set(
        "remove",
        lua.create_function(|lua_ctx, (kind, url): (String, Value)| {
            let path = lua_to_path(lua_ctx, url)?;
            let result = match kind.as_str() {
                "dir" => fs_remove_dir(&path),
                "dir_all" => fs_remove_dir_all(&path),
                "dir_clean" => clean_dir(&path),
                _ => fs_remove_file(&path),
            };
            result.map_err(|e| mlua::Error::RuntimeError(format!("remove failed: {e}")))
        })?,
    )?;

    fs.set(
        "rename",
        lua.create_function(|lua_ctx, (from, to): (Value, Value)| {
            let from = lua_to_path(lua_ctx, from)?;
            let to = lua_to_path(lua_ctx, to)?;
            fs_rename(&from, &to)
                .map_err(|e| mlua::Error::RuntimeError(format!("rename failed: {e}")))
        })?,
    )?;

    fs.set(
        "copy",
        lua.create_function(|lua_ctx, (from, to): (Value, Value)| {
            let from = lua_to_path(lua_ctx, from)?;
            let to = lua_to_path(lua_ctx, to)?;
            fs_copy(&from, &to).map_err(|e| mlua::Error::RuntimeError(format!("copy failed: {e}")))
        })?,
    )?;

    fs.set(
        "read_dir",
        lua.create_function(|lua_ctx, url: Value| {
            let path = lua_to_path(lua_ctx, url)?;
            let mut files = Vec::new();
            if let Ok(rd) = std::fs::read_dir(&path) {
                for entry in rd.flatten() {
                    files.push(LuaFile::from_path(&entry.path()));
                }
            }
            Ok(files)
        })?,
    )?;

    fs.set(
        "file",
        lua.create_function(|lua_ctx, url: Value| {
            let path = lua_to_path(lua_ctx, url)?;
            Ok(LuaFile::from_path(&path))
        })?,
    )?;

    Ok(())
}

fn clean_dir(path: &std::path::Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            fs_remove_dir_all(&p)?;
        } else {
            fs_remove_file(&p)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::manager::PluginRequest;
    use crate::plugin::runtime::bindings::fs::bind;
    use tokio::sync::mpsc;

    fn bind_fs(lua: &Lua) -> Table<'_> {
        let (tx, _rx) = mpsc::channel::<PluginRequest>(1);
        bind(lua, true, tx).unwrap()
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
}
