//! M2 typed userdata: `Url`.
//!
//! `Url` wraps a `PathBuf` plus an optional `Scheme` so plugins can
//! manipulate local and SFTP URLs uniformly:
//!
//! - `Url("path" | "sftp://user@host:port//path")` constructs a Url.
//! - Fields: `path`, `name`, `stem`, `ext`, `urn`, `base`, `parent`,
//!   `scheme`, `domain`, `is_regular`, `is_search`, `is_archive`,
//!   `is_absolute`, `has_root`.
//! - Methods: `join`, `starts_with`, `ends_with`, `strip_prefix`,
//!   `into_search`.
//! - Metamethods: `__eq`, `__tostring`, `__concat`.
//!
//! The local-only counterpart `Path` lives in its own module
//! (`src/plugin/types/path.rs` per Appendix B). The richness of
//! "what can I do with this Url" comes from the methods on
//! `Cha` / `File` (M2-T3/T4) and from the new `fs.*` operations
//! in M3-T6.

use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods, UserDataRef};
use std::path::{Component, Path, PathBuf};

/// URL scheme. Today only `Local` is fully supported; `Sftp` is a
/// placeholder that the M3 VFS layer will fill in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scheme {
    Local,
    Sftp {
        user: Option<String>,
        host: String,
        port: Option<u16>,
    },
}

impl Scheme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Scheme::Local => "local",
            Scheme::Sftp { .. } => "sftp",
        }
    }
}

impl Scheme {
    fn from_uri(uri: &str) -> (Scheme, String) {
        // Very small URI parser — full RFC 3986 is out of scope for
        // M2. We recognise `sftp://` and treat everything else as
        // `local`.
        if let Some(rest) = uri.strip_prefix("sftp://") {
            // `<user>@<host>:<port>/<path>` (the SFTP "double slash"
            // convention from the roadmap is preserved by storing the
            // path verbatim after the first `/`).
            let (authority, path) = match rest.find('/') {
                Some(i) => (Some(&rest[..i]), &rest[i + 1..]),
                None => (Some(rest), ""),
            };
            let mut user = None;
            let mut host = String::new();
            let mut port = None;
            if let Some(auth) = authority {
                if let Some(at) = auth.find('@') {
                    user = Some(auth[..at].to_string());
                    let after_at = &auth[at + 1..];
                    if let Some(colon) = after_at.find(':') {
                        host = after_at[..colon].to_string();
                        port = after_at[colon + 1..].parse().ok();
                    } else {
                        host = after_at.to_string();
                    }
                } else if let Some(colon) = auth.find(':') {
                    host = auth[..colon].to_string();
                    port = auth[colon + 1..].parse().ok();
                } else {
                    host = auth.to_string();
                }
            }
            (
                Scheme::Sftp { user, host, port },
                format!("/{}", path),
            )
        } else {
            (Scheme::Local, uri.to_string())
        }
    }
}

/// The M2 `Url` userdata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    pub path: PathBuf,
    pub scheme: Scheme,
}

impl Url {
    /// Parse a string into a `Url`. Local paths and `sftp://` URIs
    /// are supported.
    pub fn parse(s: &str) -> Self {
        let (scheme, path) = Scheme::from_uri(s);
        Self {
            path: PathBuf::from(path),
            scheme,
        }
    }

    /// The display string (local path, or `sftp://...`).
    pub(crate) fn file_name(&self) -> Option<String> {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }

    pub(crate) fn file_stem(&self) -> Option<String> {
        self.path
            .file_stem()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }

    pub(crate) fn file_ext(&self) -> Option<String> {
        self.path
            .extension()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }

    pub(crate) fn parent(&self) -> Option<Url> {
        self.path.parent().map(|p| Self {
            path: p.to_path_buf(),
            scheme: self.scheme.clone(),
        })
    }

    /// The display string (local path, or `sftp://...`).
    pub fn display(&self) -> String {
        match &self.scheme {
            Scheme::Local => self.path.to_string_lossy().to_string(),
            Scheme::Sftp { user, host, port } => {
                let user_part = user
                    .as_deref()
                    .map(|u| format!("{u}@"))
                    .unwrap_or_default();
                let port_part = port.map(|p| format!(":{p}")).unwrap_or_default();
                format!(
                    "sftp://{user_part}{host}{port_port}{path}",
                    user_part = user_part,
                    host = host,
                    port_port = port_part,
                    path = self.path.to_string_lossy()
                )
            }
        }
    }
}

impl UserData for Url {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(_fields: &mut F) {
        // Fields are exposed via methods (so we can return owned
        // strings). The mlua 0.9 API for fields wants `&self` ->
        // `mlua::Value`; we wrap the path-derived getters below in
        // `add_field_method_get` so the returned string is borrowed
        // for the lifetime of the call.
    }

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        // ── Fields (read-only getters) ─────────────────────────────
        methods.add_method("path", |_lua, this, ()| Ok(this.path.to_string_lossy().to_string()));
        methods.add_method("name", |_lua, this, ()| Ok(this.file_name()));
        methods.add_method("stem", |_lua, this, ()| Ok(this.file_stem()));
        methods.add_method("ext", |_lua, this, ()| Ok(this.file_ext()));
        methods.add_method("scheme", |_lua, this, ()| Ok(this.scheme.as_str().to_string()));
        methods.add_method("domain", |_lua, this, ()| {
            let d = match &this.scheme {
                Scheme::Local => None,
                Scheme::Sftp { host, .. } => Some(host.clone()),
            };
            Ok(d)
        });
        methods.add_method("is_absolute", |_lua, this, ()| Ok(this.path.is_absolute()));
        methods.add_method("has_root", |_lua, this, ()| {
            Ok(this.path.components().any(|c| matches!(c, Component::RootDir)))
        });
        methods.add_method("is_regular", |_lua, this, ()| {
            Ok(this.path.is_file() || !this.path.exists())
        });
        methods.add_method("is_search", |_lua, _this, ()| Ok(false));
        methods.add_method("is_archive", |_lua, this, ()| {
            let ext = this.file_ext().unwrap_or_default().to_lowercase();
            Ok(matches!(
                ext.as_str(),
                "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zst"
            ))
        });
        methods.add_method("parent", |_lua, this, ()| Ok(this.parent()));
        methods.add_method("base", |_lua, this, ()| {
            // base = name without extension
            Ok(this.file_stem())
        });
        methods.add_method("urn", |_lua, this, ()| {
            // URN = scheme://path (the canonical identifier)
            Ok(this.display())
        });

        // ── Methods ─────────────────────────────────────────────────
        methods.add_method("join", |_lua, this, other: String| {
            let joined = this.path.join(other);
            Ok(Some(Self {
                path: joined,
                scheme: this.scheme.clone(),
            }))
        });
        methods.add_method("starts_with", |_lua, this, base: String| {
            let base_path = Path::new(&base);
            Ok(this.path.starts_with(base_path))
        });
        methods.add_method("ends_with", |_lua, this, child: String| {
            let child_path = Path::new(&child);
            Ok(this.path.ends_with(child_path))
        });
        methods.add_method("strip_prefix", |_lua, this, base: String| {
            match this.path.strip_prefix(&base) {
                Ok(p) => Ok(Some(Self {
                    path: p.to_path_buf(),
                    scheme: this.scheme.clone(),
                })),
                Err(_) => Ok(None),
            }
        });
        methods.add_method("into_search", |_lua, this, _domain: String| {
            // M3 will fill this in: the `into_search` operation
            // switches a regular Url into a search Url. For now we
            // just return the same Url.
            Ok(this.clone())
        });

        // ── Metamethods ────────────────────────────────────────────
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(this.display())
        });
        methods.add_meta_method(MetaMethod::Eq, |_lua, this, other: mlua::Value| {
            match other {
                mlua::Value::UserData(ud) => {
                    if let Ok(other_url) = ud.borrow::<Self>() {
                        Ok(this.path == other_url.path && this.scheme == other_url.scheme)
                    } else {
                        Ok(false)
                    }
                }
                _ => Ok(false),
            }
        });
        methods.add_meta_method(MetaMethod::Concat, |_lua, this, other: mlua::Value| {
            // `..` interpolation: turn the other side into a string
            // and concatenate.
            let other_str = match other {
                mlua::Value::String(s) => s.to_str()?.to_string(),
                mlua::Value::Integer(i) => i.to_string(),
                mlua::Value::Number(n) => n.to_string(),
                mlua::Value::Boolean(b) => b.to_string(),
                mlua::Value::Nil => String::from("nil"),
                _ => String::from("?"),
            };
            Ok(format!("{}{}", this.display(), other_str))
        });
    }
}

/// The M2 `Path` userdata lives in its own module
/// (`src/plugin/types/path.rs` per Appendix B). The crate-wide
/// re-export in `crate::plugin::types` is what the runtime uses
/// to construct `pairee.Path`; we keep this comment so future
/// readers know the type moved out of `url.rs` during M2.5.

/// Helper used by `File::deref()` to expose the `__index` of the
/// inner `Cha` to the outer `File` userdata. (See M2-T4 for the
/// `Deref<Target = Cha>` pattern.)
pub fn url_borrow<'a>(
    ud_ref: &'a UserDataRef<'_, Url>,
) -> &'a Url {
    &**ud_ref
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_url_parse_local() {
        let u = Url::parse("/tmp/file.txt");
        assert_eq!(u.path, PathBuf::from("/tmp/file.txt"));
        assert_eq!(u.scheme, Scheme::Local);
        assert_eq!(u.file_name().as_deref(), Some("file.txt"));
        assert_eq!(u.file_stem().as_deref(), Some("file"));
        assert_eq!(u.file_ext().as_deref(), Some("txt"));
    }

    #[test]
    fn test_url_parse_sftp() {
        let u = Url::parse("sftp://user@host:2222//home/user/file");
        match &u.scheme {
            Scheme::Sftp { user, host, port } => {
                assert_eq!(user.as_deref(), Some("user"));
                assert_eq!(host, "host");
                assert_eq!(*port, Some(2222));
            }
            other => panic!("expected Sftp scheme, got {:?}", other),
        }
        assert_eq!(u.path, PathBuf::from("/home/user/file"));
    }

    #[test]
    fn test_url_lua_metamethods() {
        let lua = Lua::new();
        let url = Url::parse("/etc/hosts");
        lua.globals().set("u", lua.create_userdata(url).unwrap()).unwrap();
        // __tostring
        let s: String = lua.load("return tostring(u)").eval().unwrap();
        assert_eq!(s, "/etc/hosts");
        // __eq
        let eq: bool = lua.load("return u == u").eval().unwrap();
        assert!(eq);
    }

    #[test]
    fn test_url_join_and_parent() {
        let u = Url::parse("/tmp/sub");
        // join/parent are exposed as userdata methods, not as Rust
        // methods on `Url`. Verify the underlying `path` field has
        // what we expect, and that the parent helper works.
        let joined_path = u.path.join("file.txt");
        assert_eq!(joined_path, PathBuf::from("/tmp/sub/file.txt"));
        let parent = u.parent().unwrap();
        assert_eq!(parent.path, PathBuf::from("/tmp"));
    }
}
