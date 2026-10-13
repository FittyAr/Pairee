//! `Url` userdata: a local path or a remote location (`sftp://host/path`,
//! `ftp://user@host:21/dir`, ...) split into scheme, domain and path. Pure
//! string handling: building or comparing a `Url` never touches the disk
//! or the network.
//!
//! ```lua
//! local u = pairee.Url("sftp://me@box:22//srv/www/index.html")
//! u.scheme, u.domain, tostring(u.path)   -- "sftp", "me@box:22", "/srv/www/index.html"
//! u.name, u.stem, u.ext, u.parent, u.is_regular, u.is_absolute, u.has_root
//! u:join("x"), u:starts_with(base), u:ends_with("index.html"), u:strip_prefix(base)
//! ```
//!
//! A local path has scheme `"file"` and no domain (`file://` URLs are
//! accepted). In remote URLs `host//abs` and `host/abs` both mean `/abs`.

use super::path::{LuaPath, add_part_fields, concat_operand, path_arg};
use mlua::{Lua, MetaMethod, UserData, UserDataFields, UserDataMethods, Value};
use std::fmt;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LuaUrl {
    /// `None` for a local path.
    pub scheme: Option<String>,
    /// Host part of a remote URL (with user and port as written).
    pub domain: Option<String>,
    pub path: PathBuf,
}

/// `scheme` of `text` when it starts with `scheme://` (two characters at
/// least, so `C://x` stays a Windows path).
fn split_scheme(text: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = text.split_once("://")?;
    let valid = scheme.len() >= 2
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    valid.then_some((scheme, rest))
}

impl LuaUrl {
    pub fn local(path: PathBuf) -> Self {
        Self {
            scheme: None,
            domain: None,
            path,
        }
    }

    pub fn parse(text: &str) -> Self {
        let Some((scheme, rest)) = split_scheme(text) else {
            return Self::local(PathBuf::from(text));
        };
        if scheme.eq_ignore_ascii_case("file") {
            return Self::local(PathBuf::from(rest));
        }
        let (domain, path) = rest.split_once('/').unwrap_or((rest, ""));
        let path = format!("/{}", path.trim_start_matches('/'));
        Self {
            scheme: Some(scheme.to_ascii_lowercase()),
            domain: Some(domain.to_string()),
            path: PathBuf::from(path),
        }
    }

    fn is_remote(&self) -> bool {
        self.scheme.is_some()
    }

    /// The same location with another path.
    fn with_path(&self, path: PathBuf) -> Self {
        Self {
            path,
            ..self.clone()
        }
    }

    /// The path part as text (`/` separated for remote URLs).
    fn path_text(&self) -> String {
        let text = self.path.to_string_lossy();
        if self.is_remote() {
            text.replace('\\', "/")
        } else {
            text.into_owned()
        }
    }
}

impl fmt::Display for LuaUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.scheme, &self.domain) {
            (Some(scheme), domain) => write!(
                f,
                "{scheme}://{}{}",
                domain.as_deref().unwrap_or(""),
                self.path_text()
            ),
            (None, _) => f.write_str(&self.path_text()),
        }
    }
}

/// A `Url` from any value: a `Url`, a string, a `Path` or a `File`.
fn url_arg(value: &Value) -> mlua::Result<LuaUrl> {
    match value {
        Value::String(s) => Ok(LuaUrl::parse(&s.to_string_lossy())),
        Value::UserData(ud) if ud.borrow::<LuaUrl>().is_ok() => Ok(ud.borrow::<LuaUrl>()?.clone()),
        other => Ok(LuaUrl::local(path_arg(other)?)),
    }
}

impl UserData for LuaUrl {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        add_part_fields(fields, |this: &Self| this.path.as_path());
        fields.add_field_method_get("scheme", |_, this| {
            Ok(this.scheme.clone().unwrap_or_else(|| "file".to_string()))
        });
        fields.add_field_method_get("domain", |_, this| Ok(this.domain.clone()));
        fields.add_field_method_get("path", |_, this| Ok(LuaPath(this.path.clone())));
        fields.add_field_method_get("is_regular", |_, this| Ok(!this.is_remote()));
        fields.add_field_method_get("is_absolute", |_, this| {
            Ok(if this.is_remote() {
                this.path.has_root()
            } else {
                this.path.is_absolute()
            })
        });
        fields.add_field_method_get("parent", |_, this| {
            Ok(this.path.parent().map(|p| this.with_path(p.to_path_buf())))
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("join", |_, this, child: Value| {
            Ok(this.with_path(this.path.join(path_arg(&child)?)))
        });
        methods.add_method("starts_with", |_, this, base: Value| {
            let base = url_arg(&base)?;
            Ok(base.scheme == this.scheme
                && base.domain == this.domain
                && this.path.starts_with(&base.path))
        });
        methods.add_method("ends_with", |_, this, child: Value| {
            Ok(this.path.ends_with(path_arg(&child)?))
        });
        methods.add_method("strip_prefix", |_, this, base: Value| {
            let base = url_arg(&base)?;
            let same_place = base.scheme == this.scheme && base.domain == this.domain;
            Ok(same_place
                .then(|| this.path.strip_prefix(&base.path).ok())
                .flatten()
                .map(|p| LuaPath(p.to_path_buf())))
        });
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(this.to_string()));
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: Value| {
            Ok(url_arg(&other).is_ok_and(|u| u == *this))
        });
        methods.add_meta_function(MetaMethod::Concat, |_, (a, b): (Value, Value)| {
            Ok(concat_operand(&a)? + &concat_operand(&b)?)
        });
    }
}

/// `pairee.Url(value)`.
pub fn bind(lua: &Lua) -> mlua::Result<mlua::Function> {
    lua.create_function(|_, value: Value| url_arg(&value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lua() -> Lua {
        let lua = Lua::new();
        lua.globals().set("Url", bind(&lua).unwrap()).unwrap();
        lua.globals()
            .set("Path", super::super::path::bind(&lua).unwrap())
            .unwrap();
        lua
    }

    #[test]
    fn remote_urls_split_into_parts() {
        let lua = lua();
        let (scheme, domain, path, name, regular, absolute): (
            String,
            String,
            String,
            String,
            bool,
            bool,
        ) = lua
            .load(
                r#"local u = Url("SFTP://me@box:22//srv/www/index.html")
                   return u.scheme, u.domain, tostring(u.path):gsub("\\", "/"), u.name,
                          u.is_regular, u.is_absolute"#,
            )
            .eval()
            .unwrap();
        assert_eq!(
            (
                scheme.as_str(),
                domain.as_str(),
                path.as_str(),
                name.as_str()
            ),
            ("sftp", "me@box:22", "/srv/www/index.html", "index.html")
        );
        assert!(!regular && absolute);
        let (text, parent, joined): (String, String, String) = lua
            .load(
                r#"local u = Url("ftp://host/dir/a.txt")
                   return tostring(u), tostring(u.parent), tostring(u.parent:join("b"))"#,
            )
            .eval()
            .unwrap();
        assert_eq!(text, "ftp://host/dir/a.txt");
        assert_eq!(parent, "ftp://host/dir");
        assert_eq!(joined, "ftp://host/dir/b");
    }

    #[test]
    fn local_paths_and_comparisons() {
        let lua = lua();
        let (scheme, regular, same, other_host, starts, stripped): (
            String,
            bool,
            bool,
            bool,
            bool,
            String,
        ) = lua
            .load(
                r#"local u = Url("dir/a.txt")
                   local r = Url("sftp://h/x/y")
                   return u.scheme, u.is_regular, u == Url("file://dir/a.txt"),
                          r:starts_with("sftp://other/x"), r:starts_with(Url("sftp://h/x")),
                          tostring(r:strip_prefix("sftp://h/x"))"#,
            )
            .eval()
            .unwrap();
        assert_eq!(scheme, "file");
        assert!(regular && same && !other_host && starts);
        assert_eq!(stripped, "y");
        let windows_drive = LuaUrl::parse("C://x");
        assert!(windows_drive.scheme.is_none());
        let from_path: String = lua.load(r#"return Url(Path("q")) .. "!""#).eval().unwrap();
        assert_eq!(from_path, "q!");
    }
}
