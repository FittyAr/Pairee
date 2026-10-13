//! `---` annotations at the top of a plugin's `main.lua`.
//!
//! ```lua
//! --- @since 1.2.0
//! --- @sync entry
//! --- @sync peek
//! local M = {}
//! ```
//!
//! * `@since <version>`: the Lua API version (`pairee._lua_api_version`)
//!   the plugin needs. A plugin asking for a newer API than this Pairee
//!   provides is refused at load time, with a clear error instead of a
//!   `nil` call later. (`min_pairee` in the manifest stays the check on
//!   the Pairee version.)
//! * `@sync <method>`: before the host calls `entry`, `peek`, `preload` or
//!   `seek`, it refreshes `pairee.cx` from the main loop (what
//!   `pairee.sync` does), so the method reads the panels as they are at
//!   call time without wrapping its body in `pairee.sync`.
//!
//! Only the leading block of comment and blank lines is read; annotations
//! further down are ignored, so evaluating the file is not needed.

use std::collections::BTreeSet;

/// Methods `@sync` may name.
pub const SYNC_METHODS: &[&str] = &["entry", "peek", "preload", "seek"];

/// What the annotations of one `main.lua` ask for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Annotations {
    /// Methods that get a fresh `pairee.cx` before each call.
    pub sync: BTreeSet<String>,
    /// Minimum Lua API version (`@since`).
    pub since: Option<String>,
}

impl Annotations {
    /// Reads the annotations of `main.lua`'s leading comment block.
    /// Unknown tags and `@sync` targets are reported with a warning.
    pub fn parse(source: &str) -> Self {
        let mut found = Self::default();
        let lines = source
            .lines()
            .map(str::trim)
            .take_while(|l| l.is_empty() || l.starts_with("--"));
        for line in lines {
            let Some(rest) = line.strip_prefix("---") else {
                continue;
            };
            let mut words = rest.split_whitespace();
            match (words.next(), words.next()) {
                (Some("@sync"), Some(method)) if SYNC_METHODS.contains(&method) => {
                    found.sync.insert(method.to_string());
                }
                (Some("@sync"), other) => {
                    tracing::warn!(
                        "Ignoring `--- @sync {}`: not a plugin method",
                        other.unwrap_or("")
                    );
                }
                (Some("@since"), Some(version)) => found.since = Some(version.to_string()),
                _ => {}
            }
        }
        found
    }

    /// Whether `method` is annotated `@sync`.
    pub fn is_sync(&self, method: &str) -> bool {
        self.sync.contains(method)
    }

    /// Fails when `@since` asks for a Lua API newer than `api`.
    pub fn check_since(&self, api: &str) -> anyhow::Result<()> {
        match &self.since {
            Some(since) => crate::plugin::loader::check_version_compatibility(since, api)
                .map_err(|e| anyhow::anyhow!("`--- @since {since}` (Lua API {api}): {e}")),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "\
--- @since 1.0.0
--- @sync entry
-- a plain comment
--- @sync peek

--- @sync nonsense
local M = {}
--- @sync seek
return M
";

    #[test]
    fn leading_annotations_are_read() {
        let a = Annotations::parse(SOURCE);
        assert!(a.is_sync("entry") && a.is_sync("peek"));
        assert!(!a.is_sync("seek"), "after the first code line");
        assert!(!a.is_sync("nonsense"));
        assert_eq!(a.since.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn since_is_checked_against_the_lua_api() {
        assert!(Annotations::parse(SOURCE).check_since("1.2.0").is_ok());
        let newer = Annotations::parse("--- @since 9.0\nreturn {}");
        let err = newer.check_since("1.2.0").unwrap_err().to_string();
        assert!(err.contains("@since 9.0"), "{err}");
        assert!(Annotations::default().check_since("1.2.0").is_ok());
        assert!(
            Annotations::parse("--- @since soon")
                .check_since("1.2.0")
                .is_err()
        );
    }
}
