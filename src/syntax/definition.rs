//! The language definition format: one TOML file per language, shipped in
//! the `syntax/` folder of the repository and optionally dropped by users
//! into the `syntax/` folder of the configuration directory.
//!
//! ```toml
//! name = "Rust"
//! extensions = ["rs"]
//!
//! [[rule]]
//! kind = "keyword"
//! words = ["fn", "let"]
//!
//! [[region]]
//! kind = "string"
//! start = '"'
//! end = '"'
//! skip = '\\.'
//! ```
//!
//! See `help/en/configuration_details.md` for the full description.

use super::SyntaxKind;
use serde::Deserialize;

/// One language, as written in its file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    /// Shown in the editor status line; a user file with the name of a
    /// built-in language replaces it.
    pub name: String,
    /// File extensions without the dot (compared case-insensitively).
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Whole file names, such as `Makefile` (case-insensitive).
    #[serde(default)]
    pub names: Vec<String>,
    /// Regular expression tried on the file name.
    #[serde(default)]
    pub filename: Option<String>,
    /// Regular expression tried on the first line (shebangs, `<?xml`).
    #[serde(default)]
    pub first_line: Option<String>,
    /// Match every pattern of the language ignoring case.
    #[serde(default)]
    pub case_insensitive: bool,
    /// Patterns colored anywhere outside regions.
    #[serde(default, rename = "rule")]
    pub rules: Vec<RuleDef>,
    /// Stretches between a start and an end pattern, possibly spanning lines.
    #[serde(default, rename = "region")]
    pub regions: Vec<RegionDef>,
}

/// A pattern painted with one kind. Later rules paint over earlier ones.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleDef {
    pub kind: SyntaxKind,
    /// Regular expression (the `regex` crate syntax).
    #[serde(default)]
    pub pattern: Option<String>,
    /// Shorthand for a pattern matching any of these whole words.
    #[serde(default)]
    pub words: Vec<String>,
    /// Paint only this capture group of `pattern` (0, the default, is the
    /// whole match).
    #[serde(default)]
    pub group: usize,
}

/// A region: `start` opens it, `end` closes it (on the same or a later
/// line), and `skip` matches are passed over while looking for the end
/// (escaped quotes). Inside, only its own rules and regions apply.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionDef {
    /// Color of the whole region (`None`: only its inner rules color it).
    #[serde(default)]
    pub kind: Option<SyntaxKind>,
    pub start: String,
    pub end: String,
    #[serde(default)]
    pub skip: Option<String>,
    #[serde(default, rename = "rule")]
    pub rules: Vec<RuleDef>,
    #[serde(default, rename = "region")]
    pub regions: Vec<RegionDef>,
}

impl Definition {
    /// Parses the text of a definition file.
    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_tables() {
        let def = Definition::parse(
            r#"
            name = "X"
            extensions = ["x"]
            [[rule]]
            kind = "keyword"
            words = ["if"]
            [[region]]
            kind = "string"
            start = '"'
            end = '"'
            [[region.rule]]
            kind = "constant"
            pattern = '\\.'
            [[region.region]]
            start = '\{'
            end = '\}'
            "#,
        )
        .unwrap();
        assert_eq!(def.rules[0].kind, SyntaxKind::Keyword);
        assert_eq!(def.regions[0].rules[0].kind, SyntaxKind::Constant);
        assert!(def.regions[0].regions[0].kind.is_none());
    }

    #[test]
    fn rejects_unknown_kinds_and_fields() {
        let bad_kind = "name = \"X\"\n[[rule]]\nkind = \"nope\"\npattern = \"a\"";
        assert!(Definition::parse(bad_kind).is_err());
        assert!(Definition::parse("name = \"X\"\ncolour = 1").is_err());
    }
}
