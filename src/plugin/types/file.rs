//! M2 typed userdata: `File` — the main entry point for plugin
//! code that needs to work with file entries.
//!
//! `File` derefs to its inner `Cha`, so plugin code can call
//! `cha:perm()` or read `cha.is_dir` directly on a `File` value.

use super::cha::{Cha, ChaKind, ChaMode};
use super::url::Url;
use mlua::{MetaMethod, UserData, UserDataMethods};
use std::ops::Deref;

/// The M2 `File` userdata.
#[derive(Debug, Clone)]
pub struct File {
    pub url: Url,
    pub cha: Cha,
    pub link_to: Option<Url>,
}

impl File {
    /// Build a `File` from a URL plus the on-disk metadata.
    pub fn from_url_and_metadata(url: Url, meta: std::fs::Metadata, follow: bool) -> Self {
        let cha = Cha::from_metadata(&meta, follow);
        let link_to = if cha.mode.contains(ChaMode::T_LINK) {
            // Read the symlink target; ignore errors (we just leave
            // link_to as None).
            std::fs::read_link(url.path.as_path())
                .ok()
                .map(|p| Url::parse(&p.to_string_lossy()))
        } else {
            None
        };
        Self {
            url,
            cha,
            link_to,
        }
    }

    /// Build a `File` for a URL whose on-disk metadata we have not
    /// yet inspected (used by the registry peek path which only has
    /// the path string).
    pub fn from_url(url: Url) -> Self {
        Self {
            url,
            cha: Cha::dummy(),
            link_to: None,
        }
    }

    /// MIME type derived from the file extension (very small table;
    /// M3's `fs.cha(url)` will use the `infer` crate for content
    /// sniffing).
    pub fn mime(&self) -> Option<String> {
        let ext = self.url.file_ext()?.to_lowercase();
        let m = match ext.as_str() {
            "txt" | "log" | "md" | "rst" => "text/plain",
            "rs" => "text/rust",
            "py" => "text/x-python",
            "js" | "mjs" => "text/javascript",
            "ts" => "text/typescript",
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "json" => "application/json",
            "toml" => "application/toml",
            "yaml" | "yml" => "application/yaml",
            "xml" => "application/xml",
            "pdf" => "application/pdf",
            "zip" => "application/zip",
            "tar" => "application/x-tar",
            "gz" => "application/gzip",
            "7z" => "application/x-7z-compressed",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "mp3" => "audio/mpeg",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            _ => return None,
        };
        Some(m.to_string())
    }

    /// One-line size label (`"1.2K"`, `"3.4M"`, …). The M2
    /// implementation is intentionally tiny — M3 will plug in the
    /// same helper Pairee uses internally so the two stay in sync.
    pub fn size_label(&self) -> String {
        crate_size_label(self.cha.len)
    }
}

impl Deref for File {
    type Target = Cha;
    fn deref(&self) -> &Self::Target {
        &self.cha
    }
}

impl UserData for File {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        // ── Fields ────────────────────────────────────────────────
        methods.add_method("cha", |_lua, this, ()| Ok(this.cha.clone()));
        methods.add_method("url", |_lua, this, ()| Ok(this.url.clone()));
        methods.add_method("link_to", |_lua, this, ()| Ok(this.link_to.clone()));
        methods.add_method("name", |_lua, this, ()| Ok(this.url.file_name()));
        methods.add_method("path", |_lua, this, ()| Ok(this.url.path.to_string_lossy().to_string()));
        methods.add_method("cache", |_lua, this, ()| {
            // `cache` is a placeholder for the M3 cache URL helper.
            // For M2 we just return a hash-based string under the
            // user's cache dir so plugins can write/derive content
            // without depending on the full M3 plumbing.
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut h = DefaultHasher::new();
            this.url.path.hash(&mut h);
            let cache = crate::config::paths::get_cache_dir()
                .join("preview_cache")
                .join(format!("{:016x}", h.finish()));
            Ok(cache.to_string_lossy().to_string())
        });

        // ── Methods ───────────────────────────────────────────────
        methods.add_method("icon", |_lua, _this, ()| Ok(" ".to_string()));
        methods.add_method("size", |_lua, this, ()| Ok(this.cha.len));
        methods.add_method("mime", |_lua, this, ()| Ok(this.mime()));
        methods.add_method("prefix", |_lua, this, ()| Ok(this.url.file_name()));
        methods.add_method("style", |_lua, _this, ()| Ok(String::new()));
        methods.add_method("is_selected", |_lua, _this, ()| Ok(false));
        methods.add_method("is_yanked", |_lua, _this, ()| Ok(false));
        methods.add_method("found", |_lua, _this, ()| Ok(true));
        methods.add_method("hash", |_lua, this, ()| {
            match this.cha.hash(&this.url.path, false) {
                Ok(h) => Ok(h),
                Err(_) => Ok(String::new()),
            }
        });

        // ── Metamethods ───────────────────────────────────────────
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(this.url.display())
        });
        methods.add_meta_method(MetaMethod::Eq, |_lua, this, other: mlua::Value| {
            if let mlua::Value::UserData(ud) = other {
                if let Ok(other_file) = ud.borrow::<File>() {
                    return Ok(this.url == other_file.url);
                }
            }
            Ok(false)
        });
    }
}

/// Tiny `12K`/`3.4M`/`1.2G` formatter, kept private here so the
/// `File::size_label` API matches the size labels in the main UI.
fn crate_size_label(n: u64) -> String {
    const K: f64 = 1024.0;
    const M: f64 = K * 1024.0;
    const G: f64 = M * 1024.0;
    const T: f64 = G * 1024.0;
    let n = n as f64;
    if n >= T {
        format!("{:.1}T", n / T)
    } else if n >= G {
        format!("{:.1}G", n / G)
    } else if n >= M {
        format!("{:.1}M", n / M)
    } else if n >= K {
        format!("{:.1}K", n / K)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;
    use std::path::PathBuf;

    fn test_file() -> File {
        let url = Url {
            path: PathBuf::from("/tmp/test.txt"),
            scheme: super::super::url::Scheme::Local,
        };
        File {
            url,
            cha: Cha {
                mode: ChaMode(0o644 | ChaMode::T_FILE.0),
                kind: ChaKind(0),
                len: 1234,
                atime: None,
                btime: None,
                mtime: None,
                uid: Some(1000),
                gid: Some(1000),
                nlink: 1,
            },
            link_to: None,
        }
    }

    #[test]
    fn test_size_label() {
        let f = test_file();
        assert_eq!(f.size_label(), "1.2K");
    }

    #[test]
    fn test_mime_from_extension() {
        let f = test_file();
        assert_eq!(f.mime().as_deref(), Some("text/plain"));
    }

    #[test]
    fn test_file_deref_to_cha() {
        let f = test_file();
        // Deref: File -> Cha, so we can call Cha methods directly.
        let mode = f.mode;
        assert!(mode.contains(ChaMode::T_FILE));
        assert_eq!(f.len, 1234);
    }

    #[test]
    fn test_file_lua_methods() {
        let lua = Lua::new();
        let f = test_file();
        let f_ud = lua.create_userdata(f).unwrap();
        // Store under a name and exercise direct (non-deref) methods
        // via a Lua function. The Deref<Cha> pattern means Cha
        // methods (`is_dir`, `len`, …) are reachable through
        // `f.cha:is_dir()` in plugins.
        let table = lua.create_table().unwrap();
        table.set("f", f_ud).unwrap();
        lua.globals().set("t", table).unwrap();
        let size: u64 = lua.load("return t.f:size()").eval().unwrap();
        assert_eq!(size, 1234);
        let mime: Option<String> = lua.load("return t.f:mime()").eval().unwrap();
        assert_eq!(mime.as_deref(), Some("text/plain"));
        // Cha methods reachable through the `cha()` accessor.
        let is_dir: bool = lua.load("return t.f:cha():is_dir()").eval().unwrap();
        assert!(!is_dir);
    }
}
