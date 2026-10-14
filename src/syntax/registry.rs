//! The languages Pairee knows: the built-in definitions plus the user's
//! files in the `syntax/` folder of the configuration directory, which are
//! tried first and replace a built-in language of the same name.

use super::builtin;
use super::definition::Definition;
use super::language::Language;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, PoisonError, RwLock};

/// A definition file that could not be used, with the reason.
pub type LoadError = (PathBuf, String);

#[derive(Debug, Default)]
pub struct Registry {
    /// User languages first, then the built-ins they do not replace.
    languages: Vec<Arc<Language>>,
}

impl Registry {
    pub fn new(user: Vec<Arc<Language>>) -> Self {
        let replaced = |name: &str| user.iter().any(|u| u.name().eq_ignore_ascii_case(name));
        let builtin: Vec<_> = builtin::languages()
            .iter()
            .filter(|b| !replaced(b.name()))
            .cloned()
            .collect();
        Self {
            languages: user.into_iter().chain(builtin).collect(),
        }
    }

    /// The language of `path` (whose first line is `first_line`): the best
    /// kind of match wins (file name, name pattern, extension, first line),
    /// earlier languages on ties.
    pub fn detect(&self, path: &Path, first_line: &str) -> Option<Arc<Language>> {
        self.languages
            .iter()
            .filter_map(|l| l.matches(path, first_line).map(|m| (m, l)))
            .min_by_key(|(m, _)| *m)
            .map(|(_, l)| Arc::clone(l))
    }
}

fn active_slot() -> &'static RwLock<Arc<Registry>> {
    static ACTIVE: OnceLock<RwLock<Arc<Registry>>> = OnceLock::new();
    ACTIVE.get_or_init(|| RwLock::new(Arc::new(Registry::new(Vec::new()))))
}

/// The registry in use (built-ins only until [`load_user_definitions`]).
pub fn active() -> Arc<Registry> {
    Arc::clone(&active_slot().read().unwrap_or_else(PoisonError::into_inner))
}

/// Reads every `*.toml` file of `dir` (a missing folder holds none) and
/// makes the valid ones active. Invalid files are skipped and returned.
pub fn load_user_definitions(dir: &Path) -> Vec<LoadError> {
    let (user, errors) = read_dir(dir);
    let registry = Arc::new(Registry::new(user));
    *active_slot()
        .write()
        .unwrap_or_else(PoisonError::into_inner) = registry;
    errors
}

/// The valid definitions of `dir`, in file name order, and the errors.
pub fn read_dir(dir: &Path) -> (Vec<Arc<Language>>, Vec<LoadError>) {
    let mut files: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("toml"))
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    let mut languages = Vec::new();
    let mut errors = Vec::new();
    for path in files {
        match read_file(&path) {
            Ok(language) => languages.push(Arc::new(language)),
            Err(e) => errors.push((path, e)),
        }
    }
    (languages, errors)
}

fn read_file(path: &Path) -> Result<Language, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let language = Language::new(Definition::parse(&text)?)?;
    language.check()?;
    Ok(language)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_files_override_builtins_and_report_errors() {
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, text: &str| std::fs::write(dir.path().join(name), text).unwrap();
        write(
            "rust.toml",
            "name = \"Rust\"\nextensions = [\"rs\", \"rsx\"]",
        );
        write("new.toml", "name = \"Notes\"\nextensions = [\"notes\"]");
        write("broken.toml", "name = ");
        write(
            "badregex.toml",
            "name = \"B\"\n[[rule]]\nkind = \"tag\"\npattern = '('",
        );
        write("readme.md", "not a definition");
        let (user, errors) = read_dir(dir.path());
        let failed: Vec<_> = errors.iter().map(|(p, _)| p.file_name().unwrap()).collect();
        assert_eq!(failed, ["badregex.toml", "broken.toml"]);
        assert!(errors[0].1.contains("rule 1"));
        let registry = Registry::new(user);
        let rust = registry.detect(Path::new("x.rsx"), "").unwrap();
        assert_eq!(rust.name(), "Rust");
        assert!(rust.grammar().unwrap().root.rules.is_empty(), "user's copy");
        assert!(
            registry
                .detect(Path::new("x.rs"), "")
                .is_some_and(|l| Arc::ptr_eq(&l, &rust))
        );
        let notes = registry.detect(Path::new("a.NOTES"), "").unwrap();
        assert_eq!(notes.name(), "Notes");
        assert!(registry.detect(Path::new("a.py"), "").is_some(), "built-in");
    }

    #[test]
    fn a_missing_folder_has_no_definitions() {
        let (user, errors) = read_dir(Path::new("does/not/exist"));
        assert!(user.is_empty() && errors.is_empty());
    }

    #[test]
    fn names_beat_extensions_and_first_lines() {
        let registry = Registry::new(Vec::new());
        let name = |p: &str, first: &str| {
            registry
                .detect(Path::new(p), first)
                .map(|l| l.name().to_string())
        };
        assert_eq!(name("Makefile", "").as_deref(), Some("Makefile"));
        assert_eq!(name("x.PY", "").as_deref(), Some("Python"));
        assert_eq!(name("script", "#!/bin/bash").as_deref(), Some("Shell"));
        assert_eq!(
            name("run", "#!/usr/bin/env python3").as_deref(),
            Some("Python")
        );
        assert_eq!(name("notes.txt", "hello"), None);
        assert_eq!(name("unknown.zzz", "plain"), None);
    }
}
