//! Packaged `lang/*.toml` files stay parseable and share the same translation keys.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

fn lang_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("lang")
        .join(name)
}

fn translation_keys(name: &str) -> HashSet<String> {
    let src = fs::read_to_string(lang_path(name)).unwrap_or_else(|e| {
        panic!("read {}: {e}", lang_path(name).display());
    });
    let value: toml::Value = toml::from_str(&src).unwrap_or_else(|e| {
        panic!("parse {}: {e}", lang_path(name).display());
    });
    value
        .get("translations")
        .and_then(|t| t.as_table())
        .expect("translations table")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn packaged_en_and_es_share_the_same_keys() {
    let en = translation_keys("en.toml");
    let es = translation_keys("es.toml");
    let missing_es: Vec<&String> = en.iter().filter(|k| !es.contains(*k)).collect();
    let extra_es: Vec<&String> = es.iter().filter(|k| !en.contains(*k)).collect();
    assert!(
        missing_es.is_empty() && extra_es.is_empty(),
        "packaged lang key mismatch: missing in es={missing_es:?} extra in es={extra_es:?}"
    );
    assert!(
        en.contains("tab_system") && en.contains("tab_panel"),
        "English pack must include core tab keys"
    );
}
