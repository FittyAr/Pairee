//! Key → text lookup shared by Pairee's own strings ([`t`]) and plugin
//! translations (`pairee.t`): an active dictionary with a fallback one,
//! plus `{name}` interpolation.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::types::{CURRENT_TRANSLATIONS, LanguageFile};

/// Flat key → text dictionary.
pub type Dictionary = HashMap<String, String>;

static ENGLISH_TRANSLATIONS: OnceLock<Dictionary> = OnceLock::new();

fn english() -> &'static Dictionary {
    ENGLISH_TRANSLATIONS.get_or_init(|| {
        let toml_str = include_str!("../../../lang/en.toml");
        let parsed: LanguageFile =
            toml::from_str(toml_str).expect("Failed to parse embedded en.toml");
        parsed.translations
    })
}

/// Returns the centralized default English translation for a key.
#[cfg(test)]
pub fn get_default_english_translation(key: &str) -> String {
    english()
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

/// `key` in `active`, else in `fallback`.
pub fn lookup<'a>(active: &'a Dictionary, fallback: &'a Dictionary, key: &str) -> Option<&'a str> {
    active
        .get(key)
        .or_else(|| fallback.get(key))
        .map(String::as_str)
}

/// Replaces every `{name}` in `text` with its value.
pub fn interpolate<'a>(text: &str, vars: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    vars.into_iter()
        .fold(text.to_string(), |acc, (name, value)| {
            acc.replace(&format!("{{{name}}}"), value)
        })
}

/// Flattens nested TOML tables into dotted keys (`[menu] open = "x"` →
/// `menu.open`); non-string leaves are skipped.
pub fn flatten_table(table: &toml::Table) -> Dictionary {
    fn walk(prefix: &str, table: &toml::Table, out: &mut Dictionary) {
        for (key, value) in table {
            let full = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            match value {
                toml::Value::String(s) => {
                    out.insert(full, s.clone());
                }
                toml::Value::Table(inner) => walk(&full, inner, out),
                _ => {}
            }
        }
    }
    let mut out = Dictionary::new();
    walk("", table, &mut out);
    out
}

/// Translates a key using the active language translation map.
/// Falls back to the centralized English translation if the key is not found.
pub fn t(key: &str) -> String {
    let empty = Dictionary::new();
    let guard = CURRENT_TRANSLATIONS.get().and_then(|lock| lock.read().ok());
    let active = guard.as_deref().unwrap_or(&empty);
    lookup(active, english(), key).map_or_else(|| key.to_string(), str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_prefers_active_then_fallback() {
        let active = Dictionary::from([("a".to_string(), "A!".to_string())]);
        let fallback = Dictionary::from([
            ("a".to_string(), "a".to_string()),
            ("b".to_string(), "b".to_string()),
        ]);
        assert_eq!(lookup(&active, &fallback, "a"), Some("A!"));
        assert_eq!(lookup(&active, &fallback, "b"), Some("b"));
        assert_eq!(lookup(&active, &fallback, "c"), None);
    }

    #[test]
    fn nested_tables_flatten_to_dotted_keys() {
        let table: toml::Table =
            toml::from_str("top = 'x'\n[menu]\nopen = 'Open'\nn = 3\n").unwrap();
        let flat = flatten_table(&table);
        assert_eq!(flat.get("top").map(String::as_str), Some("x"));
        assert_eq!(flat.get("menu.open").map(String::as_str), Some("Open"));
        assert!(!flat.contains_key("menu.n"));
    }

    #[test]
    fn interpolate_replaces_named_placeholders() {
        assert_eq!(
            interpolate("Hi {name}, {name}! {x}", [("name", "Ana")]),
            "Hi Ana, Ana! {x}"
        );
    }
}
