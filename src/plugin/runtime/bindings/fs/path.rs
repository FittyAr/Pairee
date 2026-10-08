//! Path sandbox + runtime-aware FS helpers.

pub use super::jail::{Access, FsPolicy};
pub use super::target::Target;
use crate::plugin::runtime::types::LuaFile;
use mlua::Value;

/// Validate `path_str` against the captured sandbox policy.
pub fn validate_path(policy: &FsPolicy, path_str: &str, access: Access) -> mlua::Result<Target> {
    policy
        .check(path_str, access)
        .map_err(mlua::Error::RuntimeError)
}

/// Accept a Lua string or `File` userdata.
pub fn lua_to_path(policy: &FsPolicy, value: Value, access: Access) -> mlua::Result<Target> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;
    use std::path::{Path, PathBuf};

    fn open_policy() -> FsPolicy {
        FsPolicy::new(Path::new("demo"), true, false)
    }

    #[test]
    fn validate_path_allows_any_for_trusted_without_secure_mode() {
        let path = validate_path(&open_policy(), "/tmp/foo", Access::Write).unwrap();
        assert_eq!(path.path(), Path::new("/tmp/foo"));
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
        assert_eq!(path.path(), PathBuf::from("/a/b"));
    }

    #[test]
    fn lua_to_path_from_file_userdata() {
        let lua = Lua::new();
        let file = LuaFile::from_path(Path::new("/tmp/x.txt"));
        let ud = lua.create_userdata(file).unwrap();
        let path = lua_to_path(&open_policy(), Value::UserData(ud), Access::Read).unwrap();
        assert_eq!(path.path(), PathBuf::from("/tmp/x.txt"));
    }
}
