//! The language definitions shipped inside the binary: the only
//! `include_str!` of `syntax/*.toml`.

use super::definition::Definition;
use super::language::Language;
use std::sync::{Arc, OnceLock};

macro_rules! definitions {
    ($($file:literal),* $(,)?) => {
        &[$(($file, include_str!(concat!("../../syntax/", $file, ".toml")))),*]
    };
}

/// File stem and text of every built-in definition.
pub const FILES: &[(&str, &str)] = definitions![
    "batch",
    "c",
    "cpp",
    "csharp",
    "css",
    "diff",
    "dockerfile",
    "go",
    "html",
    "ini",
    "java",
    "javascript",
    "json",
    "kotlin",
    "lua",
    "makefile",
    "markdown",
    "php",
    "powershell",
    "python",
    "ruby",
    "rust",
    "shell",
    "sql",
    "swift",
    "toml",
    "typescript",
    "xml",
    "yaml",
];

/// The built-in languages, parsed on first use (their grammars compile
/// later, when a file needs them).
pub fn languages() -> &'static [Arc<Language>] {
    static LANGUAGES: OnceLock<Vec<Arc<Language>>> = OnceLock::new();
    LANGUAGES.get_or_init(|| {
        FILES
            .iter()
            .filter_map(|(file, text)| {
                Definition::parse(text)
                    .and_then(Language::new)
                    .inspect_err(|e| log::error!("built-in syntax {file}: {e}"))
                    .ok()
                    .map(Arc::new)
            })
            .collect()
    })
}
