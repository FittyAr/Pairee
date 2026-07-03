//! M2 typed userdata: `Cha` (file characteristics) + the
//! `ChaKind` / `ChaMode` bitflag enums.
//!
//! `Cha` is the "everything-about-this-file-except-its-name" object.
//! A `File` derefs to a `Cha`, so plugin code can write
//! `file.is_dir`, `file.size`, `file.cha:perm()`, etc. without
//! reaching through the `cha` field.

use mlua::{MetaMethod, UserData, UserDataMethods};
use std::path::Path;
use std::time::SystemTime;

/// Bitflags describing the *kind* of a file entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChaKind(pub u32);

impl ChaKind {
    pub const FOLLOW: ChaKind = ChaKind(1 << 0); // followed a symlink to stat
    pub const HIDDEN: ChaKind = ChaKind(1 << 1); // dotfile
    pub const SYSTEM: ChaKind = ChaKind(1 << 2); // OS-managed
    pub const DUMMY: ChaKind = ChaKind(1 << 3); // placeholder (e.g. panel header)

    pub fn contains(self, flag: ChaKind) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl std::ops::BitOr for ChaKind {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Bitflags describing the *type and permission bits* of a file
/// entry. Layout is intentionally a u16 (matching `libc::mode_t`)
/// but our representation is `u32` for headroom and to make the
/// permission helpers easier to test.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChaMode(pub u32);

impl ChaMode {
    // ── Type bits (the bottom 4 nibbles) ───────────────────────────
    pub const T_FILE: ChaMode = ChaMode(0o0100000);
    pub const T_DIR: ChaMode = ChaMode(0o0040000);
    pub const T_LINK: ChaMode = ChaMode(0o0120000);
    pub const T_BLOCK: ChaMode = ChaMode(0o0060000);
    pub const T_CHAR: ChaMode = ChaMode(0o0020000);
    pub const T_FIFO: ChaMode = ChaMode(0o0010000);
    pub const T_SOCK: ChaMode = ChaMode(0o0140000);

    // ── Special permission bits ────────────────────────────────────
    pub const S_SUID: ChaMode = ChaMode(0o0004000);
    pub const S_SGID: ChaMode = ChaMode(0o0002000);
    pub const S_STICKY: ChaMode = ChaMode(0o0001000);

    // ── The 9 permission bits ──────────────────────────────────────
    pub const P_OWNER_R: ChaMode = ChaMode(0o0000400);
    pub const P_OWNER_W: ChaMode = ChaMode(0o0000200);
    pub const P_OWNER_X: ChaMode = ChaMode(0o0000100);
    pub const P_GROUP_R: ChaMode = ChaMode(0o0000040);
    pub const P_GROUP_W: ChaMode = ChaMode(0o0000020);
    pub const P_GROUP_X: ChaMode = ChaMode(0o0000010);
    pub const P_OTHER_R: ChaMode = ChaMode(0o0000004);
    pub const P_OTHER_W: ChaMode = ChaMode(0o0000002);
    pub const P_OTHER_X: ChaMode = ChaMode(0o0000001);

    pub fn contains(self, flag: ChaMode) -> bool {
        (self.0 & flag.0) != 0
    }

    /// Returns the 9-bit permission portion of the mode.
    pub fn perm_bits(self) -> u32 {
        self.0 & 0o777
    }

    /// Unix-only: render the mode as a 10-character `rwxr-xr-x`-style
    /// string. Returns `None` on Windows (where the Unix bit layout
    /// does not apply).
    #[cfg(unix)]
    pub fn perm_string(self) -> Option<String> {
        let mut out = String::with_capacity(10);
        out.push(if self.contains(ChaMode::P_OWNER_R) { 'r' } else { '-' });
        out.push(if self.contains(ChaMode::P_OWNER_W) { 'w' } else { '-' });
        out.push(if self.contains(ChaMode::P_OWNER_X) {
            if self.contains(ChaMode::S_SUID) { 's' } else { 'x' }
        } else {
            if self.contains(ChaMode::S_SUID) { 'S' } else { '-' }
        });
        out.push(if self.contains(ChaMode::P_GROUP_R) { 'r' } else { '-' });
        out.push(if self.contains(ChaMode::P_GROUP_W) { 'w' } else { '-' });
        out.push(if self.contains(ChaMode::P_GROUP_X) {
            if self.contains(ChaMode::S_SGID) { 's' } else { 'x' }
        } else {
            if self.contains(ChaMode::S_SGID) { 'S' } else { '-' }
        });
        out.push(if self.contains(ChaMode::P_OTHER_R) { 'r' } else { '-' });
        out.push(if self.contains(ChaMode::P_OTHER_W) { 'w' } else { '-' });
        out.push(if self.contains(ChaMode::P_OTHER_X) {
            if self.contains(ChaMode::S_STICKY) { 't' } else { 'x' }
        } else {
            if self.contains(ChaMode::S_STICKY) { 'T' } else { '-' }
        });
        Some(out)
    }

    #[cfg(not(unix))]
    pub fn perm_string(self) -> Option<String> {
        None
    }
}

impl std::ops::BitOr for ChaMode {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// The M2 `Cha` userdata. Constructed from a `std::fs::Metadata` (or
/// from a plain table in Lua, see the `Cha{...}` constructor).
#[derive(Debug, Clone)]
pub struct Cha {
    pub mode: ChaMode,
    pub kind: ChaKind,
    pub len: u64,
    pub atime: Option<SystemTime>,
    pub btime: Option<SystemTime>,
    pub mtime: Option<SystemTime>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub nlink: u64,
}

impl Cha {
    /// Build a `Cha` from a `std::fs::Metadata` (best effort; on
    /// Windows some fields are `None`).
    #[cfg(unix)]
    pub fn from_metadata(meta: &std::fs::Metadata, follow: bool) -> Self {
        use std::os::unix::fs::MetadataExt;
        let mode_raw = meta.mode();
        let mode = ChaMode(mode_raw);
        let kind: ChaKind = if follow { ChaKind::FOLLOW } else { ChaKind(0) };
        Self {
            mode,
            kind,
            len: meta.len(),
            atime: meta.accessed().ok(),
            btime: meta.created().ok(),
            mtime: meta.modified().ok(),
            uid: Some(meta.uid()),
            gid: Some(meta.gid()),
            nlink: meta.nlink(),
        }
    }

    #[cfg(not(unix))]
    pub fn from_metadata(meta: &std::fs::Metadata, follow: bool) -> Self {
        let kind: ChaKind = if follow { ChaKind::FOLLOW } else { ChaKind(0) };
        Self {
            mode: ChaMode(0),
            kind,
            len: meta.len(),
            atime: meta.accessed().ok(),
            btime: meta.created().ok(),
            mtime: meta.modified().ok(),
            uid: None,
            gid: None,
            nlink: 1,
        }
    }

    /// Build a placeholder `Cha` for a file that does not (yet)
    /// exist on disk (e.g. when a plugin asks for `cha` of a path
    /// it just constructed).
    pub fn dummy() -> Self {
        Self {
            mode: ChaMode::T_FILE,
            kind: ChaKind::DUMMY,
            len: 0,
            atime: None,
            btime: None,
            mtime: None,
            uid: None,
            gid: None,
            nlink: 1,
        }
    }

    /// Hash the path's contents (or the first `long?` bytes of it).
    /// Returns a 128-bit XxHash3 value as a 32-character hex string.
    pub fn hash(&self, path: &Path, long: bool) -> std::io::Result<String> {
        use std::fs::File;
        use std::io::Read;
        use xxhash_rust::xxh3::Xxh3;

        let mut file = File::open(path)?;
        let mut hasher = Xxh3::new();
        if long {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            hasher.update(&buf);
        } else {
            let mut buf = vec![0u8; 4096];
            let n = file.read(&mut buf)?;
            hasher.update(&buf[..n]);
        }
        Ok(format!("{:032x}", hasher.digest128()))
    }
}

impl UserData for Cha {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        // ── Fields ────────────────────────────────────────────────
        methods.add_method("mode", |_lua, this, ()| Ok(this.mode.0));
        methods.add_method("is_dir", |_lua, this, ()| {
            Ok(this.mode.contains(ChaMode::T_DIR))
        });
        methods.add_method("is_hidden", |_lua, this, ()| {
            // A file is "hidden" if its first component starts with `.`
            let is_hidden = false;
            // We need a path to know this; without a path, return false.
            let _ = is_hidden;
            Ok(false)
        });
        methods.add_method("is_link", |_lua, this, ()| {
            Ok(this.mode.contains(ChaMode::T_LINK))
        });
        methods.add_method("is_orphan", |_lua, this, ()| {
            Ok(this.kind.contains(ChaKind::DUMMY))
        });
        methods.add_method("is_dummy", |_lua, this, ()| {
            Ok(this.kind.contains(ChaKind::DUMMY))
        });
        methods.add_method("is_block", |_lua, this, ()| {
            Ok(this.mode.contains(ChaMode::T_BLOCK))
        });
        methods.add_method("is_char", |_lua, this, ()| {
            Ok(this.mode.contains(ChaMode::T_CHAR))
        });
        methods.add_method("is_fifo", |_lua, this, ()| {
            Ok(this.mode.contains(ChaMode::T_FIFO))
        });
        methods.add_method("is_sock", |_lua, this, ()| {
            Ok(this.mode.contains(ChaMode::T_SOCK))
        });
        methods.add_method("is_exec", |_lua, this, ()| {
            Ok(this.mode.contains(ChaMode::P_OWNER_X))
        });
        methods.add_method("is_sticky", |_lua, this, ()| {
            Ok(this.mode.contains(ChaMode::S_STICKY))
        });
        methods.add_method("len", |_lua, this, ()| Ok(this.len));
        methods.add_method("atime", |_lua, this, ()| Ok(system_time_to_secs(this.atime)));
        methods.add_method("btime", |_lua, this, ()| Ok(system_time_to_secs(this.btime)));
        methods.add_method("mtime", |_lua, this, ()| Ok(system_time_to_secs(this.mtime)));
        methods.add_method("uid", |_lua, this, ()| Ok(this.uid));
        methods.add_method("gid", |_lua, this, ()| Ok(this.gid));
        methods.add_method("nlink", |_lua, this, ()| Ok(this.nlink));

        // ── Methods ───────────────────────────────────────────────
        methods.add_method("perm", |_lua, this, ()| Ok(this.mode.perm_string()));

        // ── Metamethods ───────────────────────────────────────────
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(format!(
                "Cha(mode={:o}, len={}, nlink={})",
                this.mode.0, this.len, this.nlink
            ))
        });
    }
}

fn system_time_to_secs(t: Option<SystemTime>) -> Option<f64> {
    t.and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_perm_string_unix() {
        // 0o755 = rwxr-xr-x
        let m = ChaMode(0o755);
        assert_eq!(m.perm_string().as_deref(), Some("rwxr-xr-x"));
    }

    #[test]
    fn test_perm_string_with_suid() {
        // 0o4755 = rwsr-xr-x
        let m = ChaMode(0o4755);
        assert_eq!(m.perm_string().as_deref(), Some("rwsr-xr-x"));
    }

    #[test]
    fn test_cha_lua_methods() {
        let lua = Lua::new();
        let cha = Cha {
            mode: ChaMode(0o755 | ChaMode::T_FILE.0),
            kind: ChaKind(0),
            len: 42,
            atime: None,
            btime: None,
            mtime: None,
            uid: Some(1000),
            gid: Some(1000),
            nlink: 1,
        };
        lua.globals().set("cha", lua.create_userdata(cha).unwrap()).unwrap();
        let is_dir: bool = lua.load("return cha:is_dir()").eval().unwrap();
        assert!(!is_dir);
        let len: u64 = lua.load("return cha:len()").eval().unwrap();
        assert_eq!(len, 42);
        let perm: Option<String> = lua.load("return cha:perm()").eval().unwrap();
        assert_eq!(perm.as_deref(), Some("rwxr-xr-x"));
    }

    #[test]
    fn test_cha_dummy_lua() {
        let lua = Lua::new();
        let cha = Cha::dummy();
        lua.globals().set("cha", lua.create_userdata(cha).unwrap()).unwrap();
        let is_orphan: bool = lua.load("return cha:is_orphan()").eval().unwrap();
        assert!(is_orphan);
    }
}
