//! Path sandbox + runtime-aware FS helpers.

use crate::plugin::runtime::types::LuaFile;
use mlua::{Lua, Value};
use std::path::{Path, PathBuf};

pub fn is_secure_mode(lua: &Lua) -> bool {
    if let Ok(pairee) = lua.globals().get::<_, mlua::Table>("pairee") {
        pairee.get::<_, bool>("_secure_mode").unwrap_or(false)
    } else {
        false
    }
}

pub fn validate_path(lua: &Lua, path_str: &str) -> mlua::Result<PathBuf> {
    let path = PathBuf::from(path_str);
    if is_secure_mode(lua) {
        let abs_path = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let workspace = std::env::current_dir().unwrap_or_default();
        let config = crate::config::paths::get_config_dir();
        let cache = crate::config::paths::get_cache_dir();

        let in_workspace = abs_path.starts_with(&workspace);
        let in_config = abs_path.starts_with(&config);
        let in_cache = abs_path.starts_with(&cache);

        if !in_workspace && !in_config && !in_cache {
            return Err(mlua::Error::RuntimeError(format!(
                "Security violation: path {:?} is outside permitted sandboxed directories in Secure Mode",
                path
            )));
        }
    }
    Ok(path)
}

/// Accept a Lua string or `File` userdata.
pub fn lua_to_path(lua: &Lua, value: Value) -> mlua::Result<PathBuf> {
    match value {
        Value::String(s) => validate_path(lua, s.to_str()?),
        Value::UserData(ud) => match ud.borrow::<LuaFile>() {
            Ok(file) => validate_path(lua, &file.path),
            Err(_) => Err(mlua::Error::RuntimeError(
                "expected a path string or File userdata".into(),
            )),
        },
        _ => Err(mlua::Error::RuntimeError(
            "expected a path string or File userdata".into(),
        )),
    }
}

/// Prefer `tokio::fs` on the multi-thread plugin worker; fall back to `std::fs`.
pub fn fs_read_to_string(path: &Path) -> std::io::Result<String> {
    with_fs(
        || async { tokio::fs::read_to_string(path).await },
        || std::fs::read_to_string(path),
    )
}

pub fn fs_write(path: &Path, data: &str) -> std::io::Result<()> {
    with_fs(
        || async { tokio::fs::write(path, data).await },
        || std::fs::write(path, data),
    )
}

pub fn fs_create_dir(path: &Path) -> std::io::Result<()> {
    with_fs(
        || async { tokio::fs::create_dir(path).await },
        || std::fs::create_dir(path),
    )
}

pub fn fs_create_dir_all(path: &Path) -> std::io::Result<()> {
    with_fs(
        || async { tokio::fs::create_dir_all(path).await },
        || std::fs::create_dir_all(path),
    )
}

pub fn fs_remove_file(path: &Path) -> std::io::Result<()> {
    with_fs(
        || async { tokio::fs::remove_file(path).await },
        || std::fs::remove_file(path),
    )
}

pub fn fs_remove_dir(path: &Path) -> std::io::Result<()> {
    with_fs(
        || async { tokio::fs::remove_dir(path).await },
        || std::fs::remove_dir(path),
    )
}

pub fn fs_remove_dir_all(path: &Path) -> std::io::Result<()> {
    with_fs(
        || async { tokio::fs::remove_dir_all(path).await },
        || std::fs::remove_dir_all(path),
    )
}

pub fn fs_rename(from: &Path, to: &Path) -> std::io::Result<()> {
    with_fs(
        || async { tokio::fs::rename(from, to).await },
        || std::fs::rename(from, to),
    )
}

pub fn fs_copy(from: &Path, to: &Path) -> std::io::Result<u64> {
    with_fs(
        || async { tokio::fs::copy(from, to).await },
        || std::fs::copy(from, to),
    )
}

fn with_fs<Fut, T, Af, Sf>(async_fn: Af, sync_fn: Sf) -> std::io::Result<T>
where
    Fut: std::future::Future<Output = std::io::Result<T>>,
    Af: FnOnce() -> Fut,
    Sf: FnOnce() -> std::io::Result<T>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(|| handle.block_on(async_fn()))
        }
        _ => sync_fn(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn validate_path_allows_any_when_not_secure() {
        let lua = Lua::new();
        let path = validate_path(&lua, "/tmp/foo").unwrap();
        assert_eq!(path, PathBuf::from("/tmp/foo"));
    }

    #[test]
    fn lua_to_path_from_string() {
        let lua = Lua::new();
        let s = lua.create_string("/a/b").unwrap();
        let path = lua_to_path(&lua, Value::String(s)).unwrap();
        assert_eq!(path, PathBuf::from("/a/b"));
    }

    #[test]
    fn lua_to_path_from_file_userdata() {
        let lua = Lua::new();
        let file = LuaFile::from_path(Path::new("/tmp/x.txt"));
        let ud = lua.create_userdata(file).unwrap();
        let path = lua_to_path(&lua, Value::UserData(ud)).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/x.txt"));
    }
}
