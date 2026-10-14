//! Every built-in definition: it parses, compiles, is detected for its
//! files and colors a typical sample as expected.

use super::samples::SAMPLES;
use super::{kind_at, run};
use crate::syntax::builtin::{FILES, languages};
use crate::syntax::definition::Definition;
use crate::syntax::registry::Registry;
use std::collections::HashMap;
use std::path::Path;

#[test]
fn every_file_parses_and_compiles() {
    assert_eq!(
        languages().len(),
        FILES.len(),
        "a built-in file failed to parse"
    );
    for language in languages() {
        language
            .check()
            .unwrap_or_else(|e| panic!("{}: {e}", language.name()));
    }
}

#[test]
fn names_and_extensions_are_unique() {
    let mut claimed: HashMap<String, String> = HashMap::new();
    for (file, text) in FILES {
        let def = Definition::parse(text).unwrap_or_else(|e| panic!("{file}: {e}"));
        let keys = (def.extensions.iter().map(|e| format!("extension {e}")))
            .chain(def.names.iter().map(|n| format!("file name {n}")))
            .chain([format!("language {}", def.name)]);
        for key in keys {
            let key = key.to_lowercase();
            if let Some(other) = claimed.insert(key.clone(), file.to_string()) {
                panic!("{key} claimed by {other} and {file}");
            }
        }
    }
}

#[test]
fn every_language_has_a_sample() {
    for language in languages() {
        assert!(
            SAMPLES.iter().any(|s| s.language == language.name()),
            "no sample for {}",
            language.name()
        );
    }
}

#[test]
fn samples_are_detected_and_highlighted() {
    let registry = Registry::new(Vec::new());
    for sample in SAMPLES {
        let language = registry
            .detect(Path::new(sample.path), sample.lines[0])
            .unwrap_or_else(|| panic!("{} not detected", sample.path));
        assert_eq!(language.name(), sample.language, "{}", sample.path);
        let tokens = run(language.grammar().unwrap(), sample.lines);
        for &(line, needle, kind) in sample.expect {
            assert_eq!(
                kind_at(&tokens[line], sample.lines[line], needle),
                kind,
                "{}: {needle:?} in {:?}",
                sample.language,
                sample.lines[line]
            );
        }
    }
}
