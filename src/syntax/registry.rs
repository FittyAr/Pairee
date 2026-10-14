//! The languages Pairee knows: the built-in definitions plus the user's
//! files in the `syntax/` folder of the configuration directory, which are
//! tried first and replace a built-in language of the same name.
//!
//! The user's files are read at startup and again on request (the
//! `reload_syntax_definitions` action, or a change in the folder): a file
//! that became invalid keeps the language it defined before.

use super::builtin;
use super::definition::Definition;
use super::language::Language;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, PoisonError, RwLock};

/// A definition file that could not be used, with the reason.
pub type LoadError = (PathBuf, String);

/// A user language and the file it was read from.
pub type UserFile = (PathBuf, Arc<Language>);

#[derive(Debug, Default)]
pub struct Registry {
    /// User languages first, then the built-ins they do not replace.
    languages: Vec<Arc<Language>>,
    /// Where the user languages came from (kept by a reload when the file
    /// became invalid).
    files: Vec<UserFile>,
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
            files: Vec::new(),
        }
    }

    /// The user languages of `files` (in order) before the built-ins.
    pub fn from_files(files: Vec<UserFile>) -> Self {
        let user = files.iter().map(|(_, l)| Arc::clone(l)).collect();
        Self {
            files,
            ..Self::new(user)
        }
    }

    /// How many languages come from the user's files.
    #[cfg(test)]
    pub fn user_count(&self) -> usize {
        self.files.len()
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

    /// The language an `include` names (by name first, then by extension or
    /// file name), when its grammar compiles.
    pub fn embedded(&self, name: &str) -> Option<Arc<Language>> {
        if name.is_empty() {
            return None;
        }
        let named = |l: &&Arc<Language>| l.name().eq_ignore_ascii_case(name);
        self.languages
            .iter()
            .find(named)
            .or_else(|| self.languages.iter().find(|l| l.answers_to(name)))
            .filter(|l| l.grammar().is_some())
            .cloned()
    }

    /// The grammar a file of `language` starts in: its host's when it has
    /// one (PHP inside HTML), else its own.
    pub fn root_of(&self, language: &Arc<Language>) -> Arc<Language> {
        language
            .host()
            .and_then(|host| self.embedded(host))
            .unwrap_or_else(|| Arc::clone(language))
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
/// makes the valid ones active. Invalid files are returned; the language
/// such a file defined before (on a reload) stays.
pub fn load_user_definitions(dir: &Path) -> Vec<LoadError> {
    let (files, errors) = read_dir(dir);
    let registry = Arc::new(active().reloaded(files, &errors));
    *active_slot()
        .write()
        .unwrap_or_else(PoisonError::into_inner) = registry;
    errors
}

impl Registry {
    /// The registry of `files` freshly read, keeping this one's language
    /// of each file that failed (`errors`).
    fn reloaded(&self, mut files: Vec<UserFile>, errors: &[LoadError]) -> Self {
        let kept = self
            .files
            .iter()
            .filter(|(path, _)| errors.iter().any(|(failed, _)| failed == path));
        files.extend(kept.cloned());
        files.sort_by(|a, b| a.0.cmp(&b.0));
        Self::from_files(files)
    }
}

/// The valid definitions of `dir`, in file name order, and the errors.
pub fn read_dir(dir: &Path) -> (Vec<UserFile>, Vec<LoadError>) {
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
            Ok(language) => languages.push((path, Arc::new(language))),
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
        let registry = Registry::from_files(user);
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
    fn reloads_keep_the_previous_language_of_a_broken_file() {
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, text: &str| std::fs::write(dir.path().join(name), text).unwrap();
        write(
            "a.toml",
            "name = \"A\"
extensions = [\"aa\"]",
        );
        write(
            "b.toml",
            "name = \"B\"
extensions = [\"bb\"]",
        );
        let (files, errors) = read_dir(dir.path());
        let first = Registry::default().reloaded(files, &errors);
        assert_eq!(first.user_count(), 2);
        write("a.toml", "name = ");
        write(
            "b.toml",
            "name = \"B\"
extensions = [\"b2\"]",
        );
        let (files, errors) = read_dir(dir.path());
        assert_eq!(errors.len(), 1);
        let second = first.reloaded(files, &errors);
        let name = |r: &Registry, p: &str| r.detect(Path::new(p), "").map(|l| l.name().to_string());
        assert_eq!(name(&second, "x.aa").as_deref(), Some("A"), "kept");
        assert_eq!(name(&second, "x.b2").as_deref(), Some("B"), "updated");
        assert_eq!(name(&second, "x.bb"), None);
        std::fs::remove_file(dir.path().join("a.toml")).unwrap();
        let (files, errors) = read_dir(dir.path());
        let third = second.reloaded(files, &errors);
        assert_eq!(name(&third, "x.aa"), None, "a removed file is forgotten");
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
