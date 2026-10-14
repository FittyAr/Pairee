//! One language: how files are recognised, and its grammar compiled on
//! first use.

use super::definition::Definition;
use super::grammar::Grammar;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct Language {
    definition: Definition,
    /// Lower-case extensions and file names.
    extensions: Vec<String>,
    names: Vec<String>,
    filename: Option<Regex>,
    first_line: Option<Regex>,
    grammar: OnceLock<Option<Grammar>>,
}

/// How strongly a file matched a language, best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Match {
    Name,
    Filename,
    Extension,
    FirstLine,
}

impl Language {
    /// Prepares `definition`, compiling its detection patterns (the
    /// grammar waits for [`Self::grammar`]).
    pub fn new(definition: Definition) -> Result<Self, String> {
        let regex = |p: &Option<String>, at: &str| {
            p.as_deref()
                .map(|p| Regex::new(p).map_err(|e| format!("{at}: {e}")))
                .transpose()
        };
        let lower = |v: &[String]| v.iter().map(|s| s.to_lowercase()).collect();
        Ok(Self {
            extensions: lower(&definition.extensions),
            names: lower(&definition.names),
            filename: regex(&definition.filename, "filename")?,
            first_line: regex(&definition.first_line, "first_line")?,
            definition,
            grammar: OnceLock::new(),
        })
    }

    pub fn name(&self) -> &str {
        &self.definition.name
    }

    /// The language files of this one are written in (`host`).
    pub fn host(&self) -> Option<&str> {
        self.definition.host.as_deref()
    }

    /// Whether `name` (from an `include`) names this language: its name,
    /// one of its extensions or file names (any case).
    pub fn answers_to(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        self.definition.name.to_lowercase() == lower
            || self.extensions.contains(&lower)
            || self.names.contains(&lower)
    }

    /// The compiled grammar (`None` when a pattern does not compile; the
    /// error is logged once).
    pub fn grammar(&self) -> Option<&Grammar> {
        self.grammar
            .get_or_init(|| {
                Grammar::compile(&self.definition)
                    .inspect_err(|e| log::warn!("syntax {}: {e}", self.name()))
                    .ok()
            })
            .as_ref()
    }

    /// Compiles the grammar now, returning the error instead of logging it.
    pub fn check(&self) -> Result<(), String> {
        let grammar = Grammar::compile(&self.definition)?;
        // A concurrent first use may have compiled it already.
        let _ = self.grammar.set(Some(grammar));
        Ok(())
    }

    /// How `path` (whose first line is `first_line`) matches this language.
    pub fn matches(&self, path: &Path, first_line: &str) -> Option<Match> {
        let lower = |s: Option<&std::ffi::OsStr>| s.and_then(|s| s.to_str()).map(str::to_lowercase);
        let name = lower(path.file_name());
        if name.as_ref().is_some_and(|n| self.names.contains(n)) {
            return Some(Match::Name);
        }
        let file_name = path.file_name().and_then(|s| s.to_str());
        if let (Some(re), Some(n)) = (&self.filename, file_name)
            && re.is_match(n)
        {
            return Some(Match::Filename);
        }
        if lower(path.extension()).is_some_and(|e| self.extensions.contains(&e)) {
            return Some(Match::Extension);
        }
        self.first_line
            .as_ref()
            .filter(|re| re.is_match(first_line))
            .map(|_| Match::FirstLine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn language(extra: &str) -> Language {
        Language::new(Definition::parse(&format!("name = \"T\"\n{extra}")).unwrap()).unwrap()
    }

    #[test]
    fn matches_names_extensions_and_first_lines() {
        let lang = language(
            "extensions = [\"t\"]\nnames = [\"Tfile\"]\n\
             filename = '^Tfile\\.'\nfirst_line = '^#!.*\\bt\\b'",
        );
        assert_eq!(lang.matches(Path::new("TFILE"), ""), Some(Match::Name));
        assert_eq!(
            lang.matches(Path::new("Tfile.dev"), ""),
            Some(Match::Filename)
        );
        assert_eq!(lang.matches(Path::new("a.T"), ""), Some(Match::Extension));
        assert_eq!(
            lang.matches(Path::new("run"), "#!/usr/bin/env t"),
            Some(Match::FirstLine)
        );
        assert_eq!(lang.matches(Path::new("a.txt"), "hello"), None);
    }

    #[test]
    fn bad_patterns_are_reported() {
        let def = Definition::parse("name = \"T\"\nfirst_line = '('").unwrap();
        assert!(Language::new(def).unwrap_err().starts_with("first_line"));
        let lang = language("[[rule]]\nkind = \"tag\"\npattern = '['");
        assert!(lang.check().is_err());
        assert!(lang.grammar().is_none());
    }
}
