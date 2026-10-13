//! `Cha` userdata: the characteristics of a file (type, size, times,
//! permissions, owner), from `pairee.fs.cha(path)`, `File.cha` or built
//! with `pairee.Cha{...}`.
//!
//! Fields: `is_dir`, `is_link`, `is_orphan` (a link whose target is
//! missing), `is_hidden`, `is_exec`, `is_block`, `is_char`, `is_fifo`,
//! `is_sock`, `is_sticky`, `len`, `mtime` / `atime` / `btime` (Unix seconds
//! or `nil`), `mode`, `uid`, `gid`, `nlink` (`nil` where the platform has
//! none). Methods: `perm()` (`"drwxr-xr-x"`, `nil` on Windows) and
//! `hash(long?)` (a hex digest of size and times, to key caches).

use mlua::{Lua, UserData, UserDataFields, UserDataMethods};
use std::fs::Metadata;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct LuaCha {
    pub is_dir: bool,
    pub is_link: bool,
    pub is_orphan: bool,
    pub is_hidden: bool,
    pub is_exec: bool,
    pub is_block: bool,
    pub is_char: bool,
    pub is_fifo: bool,
    pub is_sock: bool,
    pub is_sticky: bool,
    pub len: u64,
    pub mtime: Option<u64>,
    pub atime: Option<u64>,
    pub btime: Option<u64>,
    pub mode: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub nlink: Option<u64>,
}

fn secs(time: std::io::Result<SystemTime>) -> Option<u64> {
    time.ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

impl LuaCha {
    /// Characteristics of `path` from its `symlink_metadata` (`link`, when
    /// it is a symlink) and the metadata of what it points to (`target`,
    /// `None` for a dangling link). `follow` describes the target instead.
    pub fn from_metadata(
        path: &std::path::Path,
        link: &Metadata,
        target: Option<&Metadata>,
        follow: bool,
    ) -> Self {
        let is_link = link.file_type().is_symlink();
        let meta = if follow { target.unwrap_or(link) } else { link };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut cha = Self {
            is_dir: meta.is_dir(),
            is_link,
            is_orphan: is_link && target.is_none(),
            is_hidden: crate::plugin::manager::snapshot::is_hidden_name(&name),
            is_exec: crate::plugin::manager::snapshot::is_executable(path, &name),
            len: meta.len(),
            mtime: secs(meta.modified()),
            atime: secs(meta.accessed()),
            btime: secs(meta.created()),
            ..Self::default()
        };
        cha.add_unix(meta);
        cha
    }

    #[cfg(unix)]
    fn add_unix(&mut self, meta: &Metadata) {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};
        let kind = meta.file_type();
        self.is_block = kind.is_block_device();
        self.is_char = kind.is_char_device();
        self.is_fifo = kind.is_fifo();
        self.is_sock = kind.is_socket();
        self.is_sticky = meta.mode() & 0o1000 != 0;
        self.mode = Some(meta.mode());
        self.uid = Some(meta.uid());
        self.gid = Some(meta.gid());
        self.nlink = Some(meta.nlink());
    }

    #[cfg(not(unix))]
    fn add_unix(&mut self, _meta: &Metadata) {}

    /// Reads the characteristics of `path` (`follow`: of a link's target).
    pub fn read(path: &std::path::Path, follow: bool) -> std::io::Result<Self> {
        let link = std::fs::symlink_metadata(path)?;
        let target = std::fs::metadata(path).ok();
        Ok(Self::from_metadata(path, &link, target.as_ref(), follow))
    }

    /// `ls -l` style type and permission string (`None` without a mode).
    pub fn perm(&self) -> Option<String> {
        let mode = self.mode?;
        let kind = match () {
            _ if self.is_link => 'l',
            _ if self.is_dir => 'd',
            _ if self.is_block => 'b',
            _ if self.is_char => 'c',
            _ if self.is_fifo => 'p',
            _ if self.is_sock => 's',
            _ => '-',
        };
        let mut text = String::from(kind);
        text.push_str(&crate::fs::attrs::format_unix_mode(mode));
        Some(text)
    }

    /// Hex digest of size and times (`long` adds the other fields).
    pub fn hash(&self, long: bool) -> String {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        if long {
            Hash::hash(self, &mut hasher);
        } else {
            (self.len, self.mtime, self.btime, self.is_dir).hash(&mut hasher);
        }
        format!("{:016x}", hasher.finish())
    }
}

impl UserData for LuaCha {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        macro_rules! field {
            ($($name:ident),*) => {$(
                fields.add_field_method_get(stringify!($name), |_, this| Ok(this.$name));
            )*};
        }
        field!(
            is_dir, is_link, is_orphan, is_hidden, is_exec, is_block, is_char, is_fifo, is_sock,
            is_sticky, len, mtime, atime, btime, mode, uid, gid, nlink
        );
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("perm", |_, this, ()| Ok(this.perm()));
        methods.add_method("hash", |_, this, long: Option<bool>| {
            Ok(this.hash(long.unwrap_or(false)))
        });
    }
}

/// `pairee.Cha{...}`: a `Cha` from a table of the same fields.
pub fn bind(lua: &Lua) -> mlua::Result<mlua::Function> {
    lua.create_function(|_, t: mlua::Table| {
        let flag = |name: &str| t.get::<Option<bool>>(name).map(Option::unwrap_or_default);
        Ok(LuaCha {
            is_dir: flag("is_dir")?,
            is_link: flag("is_link")?,
            is_orphan: flag("is_orphan")?,
            is_hidden: flag("is_hidden")?,
            is_exec: flag("is_exec")?,
            is_block: flag("is_block")?,
            is_char: flag("is_char")?,
            is_fifo: flag("is_fifo")?,
            is_sock: flag("is_sock")?,
            is_sticky: flag("is_sticky")?,
            len: t.get::<Option<u64>>("len")?.unwrap_or(0),
            mtime: t.get("mtime")?,
            atime: t.get("atime")?,
            btime: t.get("btime")?,
            mode: t.get("mode")?,
            uid: t.get("uid")?,
            gid: t.get("gid")?,
            nlink: t.get("nlink")?,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_describes_files_and_folders() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(".x.txt");
        std::fs::write(&file, b"abcd").unwrap();
        let cha = LuaCha::read(&file, false).unwrap();
        assert!(!cha.is_dir && !cha.is_link && cha.is_hidden);
        assert_eq!(cha.len, 4);
        assert!(cha.mtime.is_some());
        assert_eq!(cha.mode.is_some(), cfg!(unix));
        assert!(LuaCha::read(dir.path(), false).unwrap().is_dir);
        assert!(LuaCha::read(&dir.path().join("missing"), false).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn dangling_links_are_orphans() {
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("l");
        std::os::unix::fs::symlink(dir.path().join("gone"), &link).unwrap();
        let cha = LuaCha::read(&link, true).unwrap();
        assert!(cha.is_link && cha.is_orphan);
        assert!(cha.perm().unwrap().starts_with('l'));
    }

    #[test]
    fn lua_builds_and_reads_a_cha() {
        let lua = Lua::new();
        lua.globals().set("Cha", bind(&lua).unwrap()).unwrap();
        let (is_dir, len, perm, short, long): (bool, u64, String, String, String) = lua
            .load(
                r#"local c = Cha{ is_dir = true, len = 7, mode = 493, mtime = 5 }
                   return c.is_dir, c.len, c:perm(), c:hash(), c:hash(true)"#,
            )
            .eval()
            .unwrap();
        assert!(is_dir);
        assert_eq!(len, 7);
        assert_eq!(perm, "drwxr-xr-x");
        assert_eq!(short.len(), 16);
        assert_ne!(short, long);
        let none: mlua::Value = lua.load("return Cha{}:perm()").eval().unwrap();
        assert!(none.is_nil());
    }
}
