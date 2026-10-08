//! Core `pairee.fs` read/write/exists/stat/list.

use super::path::{Access, FsPolicy, fs_read_to_string, fs_write, validate_path};
use crate::plugin::runtime::types::LuaFile;
use mlua::{Lua, Table, Value};
use std::sync::Arc;

pub fn bind_core(lua: &Lua, fs: &Table<'_>, policy: &Arc<FsPolicy>) -> mlua::Result<()> {
    let p = Arc::clone(policy);
    fs.set(
        "read",
        lua.create_function(move |_, path_str: String| {
            let path = validate_path(&p, &path_str, Access::Read)?;
            fs_read_to_string(&path)
                .map_err(|e| mlua::Error::RuntimeError(format!("Failed to read file: {e}")))
        })?,
    )?;

    let p = Arc::clone(policy);
    fs.set(
        "write",
        lua.create_function(move |_, (path_str, data): (String, String)| {
            let path = validate_path(&p, &path_str, Access::Write)?;
            fs_write(&path, &data)
                .map_err(|e| mlua::Error::RuntimeError(format!("Failed to write file: {e}")))
        })?,
    )?;

    let p = Arc::clone(policy);
    fs.set(
        "exists",
        lua.create_function(move |_, path_str: String| {
            let path = validate_path(&p, &path_str, Access::Read)?;
            Ok(path.exists())
        })?,
    )?;

    let p = Arc::clone(policy);
    fs.set(
        "stat",
        lua.create_function(move |lua_ctx, path_str: String| {
            let path = validate_path(&p, &path_str, Access::Read)?;
            if !path.exists() {
                return Ok(Value::Nil);
            }
            let file = LuaFile::from_path(&path);
            Ok(Value::UserData(lua_ctx.create_userdata(file)?))
        })?,
    )?;

    let p = Arc::clone(policy);
    fs.set(
        "list",
        lua.create_function(move |_, path_str: String| {
            let path = validate_path(&p, &path_str, Access::Read)?;
            let mut entries = Vec::new();
            if let Ok(rd) = std::fs::read_dir(&path) {
                for entry in rd.flatten() {
                    entries.push(LuaFile::from_path(&entry.path()));
                }
            }
            Ok(entries)
        })?,
    )?;

    let data_dir = policy.data_dir.to_string_lossy().to_string();
    fs.set(
        "data_dir",
        lua.create_function(move |_, ()| Ok(data_dir.clone()))?,
    )?;

    Ok(())
}
