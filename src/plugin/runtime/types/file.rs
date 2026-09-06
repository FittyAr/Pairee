//! Typed `File` userdata passed to Lua plugins.

use crate::plugin::manager::snapshot::FileEntrySnapshot;
use mlua::{FromLua, MetaMethod, UserData, UserDataFields, UserDataMethods, Value};

/// A file or directory visible to a plugin (`pairee.cx.active.hovered`, etc.).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LuaFile {
    pub name: String,
    pub path: String,
    pub url: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub mime: String,
    pub mtime: Option<u64>,
    pub is_hidden: bool,
    pub is_exec: bool,
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
            mime: entry.mime.clone(),
            mtime: entry.mtime,
            is_hidden: entry.is_hidden,
            is_exec: entry.is_exec,
        }
    }

    pub fn from_path(path: &std::path::Path) -> Self {
        let path_str = path.to_string_lossy().to_string();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path_str.clone());
        let meta = std::fs::symlink_metadata(path).ok();
        let is_dir = path.is_dir();
        let modified = meta.as_ref().and_then(|m| m.modified().ok());
        Self {
            name: name.clone(),
            path: path_str.clone(),
            url: path_str,
            size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
            is_dir,
            is_symlink: meta
                .as_ref()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false),
            mime: crate::plugin::manager::snapshot::guess_mime(&name, is_dir),
            mtime: crate::plugin::manager::snapshot::mtime_unix(modified),
            is_hidden: crate::plugin::manager::snapshot::is_hidden_name(&name),
            is_exec: crate::plugin::manager::snapshot::is_executable(path, &name),
        }
    }
}

impl<'lua> FromLua<'lua> for LuaFile {
    fn from_lua(value: Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "File",
                message: Some("expected File userdata".into()),
            }),
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
        fields.add_field_method_get("mime", |_, this| Ok(this.mime.clone()));
        fields.add_field_method_get("mtime", |_, this| Ok(this.mtime));
        fields.add_field_method_get("is_hidden", |_, this| Ok(this.is_hidden));
        fields.add_field_method_get("is_exec", |_, this| Ok(this.is_exec));
    }

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(this.path.clone()));
        methods.add_meta_method(
            MetaMethod::Eq,
            |_, this, other: mlua::AnyUserData| match other.borrow::<LuaFile>() {
                Ok(other) => Ok(this.path == other.path),
                Err(_) => Ok(false),
            },
        );
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
            mime: "text/plain".into(),
            mtime: None,
            is_hidden: false,
            is_exec: false,
        }
    }

    #[test]
    fn lua_can_read_file_fields() {
        let lua = Lua::new();
        lua.globals().set("f", sample()).unwrap();
        let name: String = lua.load("return f.name").eval().unwrap();
        let size: u64 = lua.load("return f.size").eval().unwrap();
        let is_dir: bool = lua.load("return f.is_dir").eval().unwrap();
        let mime: String = lua.load("return f.mime").eval().unwrap();
        let hidden: bool = lua.load("return f.is_hidden").eval().unwrap();
        assert_eq!(name, "readme.md");
        assert_eq!(size, 12);
        assert!(!is_dir);
        assert_eq!(mime, "text/plain");
        assert!(!hidden);
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
    fn from_path_reads_metadata() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"abc").unwrap();
        let f = LuaFile::from_path(tmp.path());
        assert_eq!(f.size, 3);
        assert!(!f.is_dir);
        assert_eq!(
            f.name,
            tmp.path().file_name().unwrap().to_string_lossy().as_ref()
        );
        assert!(f.mtime.is_some());
        assert_eq!(f.mime, "application/octet-stream");
    }

    #[test]
    fn from_path_marks_dotfile_hidden_and_guesses_mime() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".hidden.txt");
        std::fs::write(&path, b"x").unwrap();
        let f = LuaFile::from_path(&path);
        assert!(f.is_hidden);
        assert_eq!(f.mime, "text/plain");
        let exec: bool = {
            let lua = Lua::new();
            lua.globals().set("f", f).unwrap();
            lua.load("return f.is_exec").eval().unwrap()
        };
        assert!(!exec);
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
            mime: "inode/directory".into(),
            mtime: Some(1),
            is_hidden: false,
            is_exec: false,
        };
        let f = LuaFile::from_snapshot(&snap);
        assert_eq!(f.name, "x");
        assert_eq!(f.path, "/x");
        assert_eq!(f.size, 3);
        assert!(f.is_dir);
        assert!(f.is_symlink);
        assert_eq!(f.mime, "inode/directory");
        assert_eq!(f.mtime, Some(1));
    }
}
