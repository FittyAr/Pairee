//! Path sandbox + runtime-aware FS helpers.

pub use super::jail::{Access, FsPolicy};
use crate::plugin::runtime::types::LuaFile;
use mlua::Value;
use std::path::{Path, PathBuf};

/// Validate `path_str` against the captured sandbox policy.
pub fn validate_path(policy: &FsPolicy, path_str: &str, access: Access) -> mlua::Result<PathBuf> {
    policy
        .check(path_str, access)
        .map_err(mlua::Error::RuntimeError)
}

/// Accept a Lua string or `File` userdata.
pub fn lua_to_path(policy: &FsPolicy, value: Value, access: Access) -> mlua::Result<PathBuf> {
    match value {
        Value::String(s) => validate_path(policy, s.to_str()?, access),
        Value::UserData(ud) => match ud.borrow::<LuaFile>() {
            Ok(file) => validate_path(policy, &file.path, access),
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

    fn open_policy() -> FsPolicy {
        FsPolicy::new(Path::new("demo"), true, false)
    }

    #[test]
    fn validate_path_allows_any_for_trusted_without_secure_mode() {
        let path = validate_path(&open_policy(), "/tmp/foo", Access::Write).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/foo"));
    }

    #[test]
    fn validate_path_rejects_untrusted_outside_jail() {
        let policy = FsPolicy::new(Path::new("demo"), false, false);
        let outside = tempfile::tempdir().unwrap();
        let p = outside.path().join("x").to_string_lossy().to_string();
        assert!(validate_path(&policy, &p, Access::Read).is_err());
    }

    #[test]
    fn lua_to_path_from_string() {
        let lua = Lua::new();
        let s = lua.create_string("/a/b").unwrap();
        let path = lua_to_path(&open_policy(), Value::String(s), Access::Read).unwrap();
        assert_eq!(path, PathBuf::from("/a/b"));
    }

    #[test]
    fn lua_to_path_from_file_userdata() {
        let lua = Lua::new();
        let file = LuaFile::from_path(Path::new("/tmp/x.txt"));
        let ud = lua.create_userdata(file).unwrap();
        let path = lua_to_path(&open_policy(), Value::UserData(ud), Access::Read).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/x.txt"));
    }
}
