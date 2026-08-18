//! Typed `File` userdata passed to Lua plugins.

use crate::plugin::manager::snapshot::FileEntrySnapshot;
use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods};

/// A file or directory visible to a plugin (`pairee.cx.active.hovered`, etc.).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LuaFile {
    pub name: String,
    pub path: String,
    pub url: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
}

impl LuaFile {
    pub fn from_snapshot(entry: &FileEntrySnapshot) -> Self {
        Self {
            name: entry.name.clone(),
            path: entry.path.clone(),
            url: entry.url.clone(),
            size: entry.size,
            is_dir: entry.is_dir,
            is_symlink: entry.is_symlink,
        }
    }
}

impl UserData for LuaFile {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.name.clone()));
        fields.add_field_method_get("path", |_, this| Ok(this.path.clone()));
        fields.add_field_method_get("url", |_, this| Ok(this.url.clone()));
        fields.add_field_method_get("size", |_, this| Ok(this.size));
        fields.add_field_method_get("is_dir", |_, this| Ok(this.is_dir));
        fields.add_field_method_get("is_symlink", |_, this| Ok(this.is_symlink));
    }

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(this.path.clone()));
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: mlua::AnyUserData| {
            match other.borrow::<LuaFile>() {
                Ok(other) => Ok(this.path == other.path),
                Err(_) => Ok(false),
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    fn sample() -> LuaFile {
        LuaFile {
            name: "readme.md".into(),
            path: "/tmp/readme.md".into(),
            url: "/tmp/readme.md".into(),
            size: 12,
            is_dir: false,
            is_symlink: false,
        }
    }

    #[test]
    fn lua_can_read_file_fields() {
        let lua = Lua::new();
        lua.globals().set("f", sample()).unwrap();
        let name: String = lua.load("return f.name").eval().unwrap();
        let size: u64 = lua.load("return f.size").eval().unwrap();
        let is_dir: bool = lua.load("return f.is_dir").eval().unwrap();
        assert_eq!(name, "readme.md");
        assert_eq!(size, 12);
        assert!(!is_dir);
    }

    #[test]
    fn lua_tostring_and_eq() {
        let lua = Lua::new();
        lua.globals().set("a", sample()).unwrap();
        lua.globals().set("b", sample()).unwrap();
        let mut other = sample();
        other.path = "/tmp/other".into();
        lua.globals().set("c", other).unwrap();
        let shown: String = lua.load("return tostring(a)").eval().unwrap();
        let same: bool = lua.load("return a == b").eval().unwrap();
        let different: bool = lua.load("return a == c").eval().unwrap();
        assert_eq!(shown, "/tmp/readme.md");
        assert!(same);
        assert!(!different);
    }

    #[test]
    fn from_snapshot_copies_identity_fields() {
        let snap = FileEntrySnapshot {
            name: "x".into(),
            url: "/x".into(),
            path: "/x".into(),
            size: 3,
            is_dir: true,
            is_symlink: true,
        };
        let f = LuaFile::from_snapshot(&snap);
        assert_eq!(f.name, "x");
        assert_eq!(f.path, "/x");
        assert_eq!(f.size, 3);
        assert!(f.is_dir);
        assert!(f.is_symlink);
    }
}
