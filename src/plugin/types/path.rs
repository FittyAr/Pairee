//! M2 typed userdata: `Path` (local filesystem path only).
//!
//! `Path` is the same shape as `Url` but always local — it has no
//! scheme concept, no SFTP support, no `__call` from string parsing,
//! and only the methods that make sense for a local `PathBuf`.
//!
//! The roadmap §5.A2 / Appendix B prescribes the dedicated file
//! `src/plugin/types/path.rs` for this type; the constructor
//! registry that exposes it to Lua (`pairee.Path`) lives in
//! `runtime::types::register`.
//!
//! Fields: `name`, `stem`, `ext`, `parent`, `is_absolute`, `has_root`.
//! Methods: `join`, `starts_with`, `ends_with`, `strip_prefix`.
//! Metamethods: `__eq`, `__tostring`, `__concat`.

use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods};
use std::path::{Component, Path, PathBuf};

/// The M2 `Path` userdata — wraps a plain local `PathBuf`.
#[derive(Debug, Clone)]
pub struct PathU {
    pub path: PathBuf,
}

impl PathU {
    pub fn from_string(s: &str) -> Self {
        Self {
            path: PathBuf::from(s),
        }
    }

    pub fn os(s: &str) -> Self {
        Self::from_string(s)
    }

    fn file_name(&self) -> Option<String> {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }
    fn file_stem(&self) -> Option<String> {
        self.path
            .file_stem()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }
    fn file_ext(&self) -> Option<String> {
        self.path
            .extension()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }
    fn parent(&self) -> Option<PathU> {
        self.path.parent().map(|p| Self {
            path: p.to_path_buf(),
        })
    }
}

impl UserData for PathU {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("name", |_lua, this, ()| Ok(this.file_name()));
        methods.add_method("stem", |_lua, this, ()| Ok(this.file_stem()));
        methods.add_method("ext", |_lua, this, ()| Ok(this.file_ext()));
        methods.add_method("parent", |_lua, this, ()| Ok(this.parent()));
        methods.add_method("is_absolute", |_lua, this, ()| Ok(this.path.is_absolute()));
        methods.add_method("has_root", |_lua, this, ()| {
            Ok(this.path.components().any(|c| matches!(c, Component::RootDir)))
        });
        methods.add_method("join", |_lua, this, other: String| {
            let joined = this.path.join(other);
            Ok(Some(Self { path: joined }))
        });
        methods.add_method("starts_with", |_lua, this, base: String| {
            Ok(this.path.starts_with(Path::new(&base)))
        });
        methods.add_method("ends_with", |_lua, this, child: String| {
            Ok(this.path.ends_with(Path::new(&child)))
        });
        methods.add_method("strip_prefix", |_lua, this, base: String| {
            match this.path.strip_prefix(&base) {
                Ok(p) => Ok(Some(Self { path: p.to_path_buf() })),
                Err(_) => Ok(None),
            }
        });

        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(this.path.to_string_lossy().to_string())
        });
        methods.add_meta_method(MetaMethod::Eq, |_lua, this, other: mlua::Value| {
            if let mlua::Value::UserData(ud) = other {
                if let Ok(other_path) = ud.borrow::<Self>() {
                    return Ok(this.path == other_path.path);
                }
            }
            Ok(false)
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
            Ok(format!("{}{}", this.path.to_string_lossy(), other_str))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_path_lua_basics() {
        let lua = Lua::new();
        let p = PathU::os("/usr/bin");
        lua.globals()
            .set("p", lua.create_userdata(p).unwrap())
            .unwrap();
        let name: Option<String> = lua.load("return p:name()").eval().unwrap();
        assert_eq!(name.as_deref(), Some("bin"));
    }

    #[test]
    fn test_path_from_string() {
        let p = PathU::from_string("/etc/hosts");
        assert_eq!(p.path, PathBuf::from("/etc/hosts"));
        assert!(p.path.is_absolute());
    }

    #[test]
    fn test_path_components() {
        let p = PathU::os("/usr/local/bin/foo.txt");
        assert_eq!(p.file_name().as_deref(), Some("foo.txt"));
        assert_eq!(p.file_stem().as_deref(), Some("foo"));
        assert_eq!(p.file_ext().as_deref(), Some("txt"));
        let parent = p.parent().unwrap();
        assert_eq!(parent.path, PathBuf::from("/usr/local/bin"));
    }

    #[test]
    fn test_path_join_strip() {
        let p = PathU::os("/tmp");
        let joined = p.path.join("sub/file.txt");
        assert_eq!(joined, PathBuf::from("/tmp/sub/file.txt"));
        let stripped = joined.strip_prefix("/tmp").unwrap();
        assert_eq!(stripped, PathBuf::from("sub/file.txt"));
        let starts = p.path.starts_with("/tmp");
        assert!(starts);
        let ends = joined.ends_with("file.txt");
        assert!(ends);
    }

    #[test]
    fn test_path_lua_methods() {
        let lua = Lua::new();
        let p = PathU::os("/usr/bin");
        lua.globals()
            .set("p", lua.create_userdata(p).unwrap())
            .unwrap();
        let name: Option<String> = lua.load("return p:name()").eval().unwrap();
        assert_eq!(name.as_deref(), Some("bin"));
        let abs: bool = lua.load("return p:is_absolute()").eval().unwrap();
        assert!(abs);
        let has_root: bool = lua.load("return p:has_root()").eval().unwrap();
        assert!(has_root);
    }

    #[test]
    fn test_path_lua_metamethods() {
        let lua = Lua::new();
        let p1 = PathU::os("/a/b");
        let p2 = PathU::os("/a/b");
        lua.globals()
            .set("p1", lua.create_userdata(p1).unwrap())
            .unwrap();
        lua.globals()
            .set("p2", lua.create_userdata(p2).unwrap())
            .unwrap();
        let eq: bool = lua.load("return p1 == p2").eval().unwrap();
        assert!(eq);
        let s: String = lua.load("return tostring(p1)").eval().unwrap();
        assert_eq!(s, "/a/b");
    }
}
