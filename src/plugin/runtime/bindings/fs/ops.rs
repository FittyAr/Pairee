//! Core `pairee.fs` read/write/exists/stat/list.

use super::path::{fs_read_to_string, fs_write, validate_path};
use crate::plugin::runtime::types::LuaFile;
use mlua::{Lua, Table, Value};

pub fn bind_core(lua: &Lua, fs: &Table<'_>) -> mlua::Result<()> {
    fs.set(
        "read",
        lua.create_function(|lua_ctx, path_str: String| {
            let path = validate_path(lua_ctx, &path_str)?;
            fs_read_to_string(&path)
                .map_err(|e| mlua::Error::RuntimeError(format!("Failed to read file: {e}")))
        })?,
    )?;

    fs.set(
        "write",
        lua.create_function(|lua_ctx, (path_str, data): (String, String)| {
            let path = validate_path(lua_ctx, &path_str)?;
            fs_write(&path, &data)
                .map_err(|e| mlua::Error::RuntimeError(format!("Failed to write file: {e}")))
        })?,
    )?;

    fs.set(
        "exists",
        lua.create_function(|lua_ctx, path_str: String| {
            let path = validate_path(lua_ctx, &path_str)?;
            Ok(path.exists())
        })?,
    )?;

    fs.set(
        "stat",
        lua.create_function(|lua_ctx, path_str: String| {
            let path = validate_path(lua_ctx, &path_str)?;
            if !path.exists() {
                return Ok(Value::Nil);
            }
            let file = LuaFile::from_path(&path);
            Ok(Value::UserData(lua_ctx.create_userdata(file)?))
        })?,
    )?;

    fs.set(
        "list",
        lua.create_function(|lua_ctx, path_str: String| {
            let path = validate_path(lua_ctx, &path_str)?;
            let mut entries = Vec::new();
            if let Ok(rd) = std::fs::read_dir(&path) {
                for entry in rd.flatten() {
                    entries.push(LuaFile::from_path(&entry.path()));
                }
            }
            Ok(entries)
        })?,
    )?;

    Ok(())
}
