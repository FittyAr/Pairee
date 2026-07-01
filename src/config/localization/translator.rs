use std::collections::HashMap;
use std::sync::OnceLock;

use super::types::{CURRENT_TRANSLATIONS, LanguageFile};

static ENGLISH_TRANSLATIONS: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Returns the centralized default English translation for a key.
pub fn get_default_english_translation(key: &str) -> String {
    let map = ENGLISH_TRANSLATIONS.get_or_init(|| {
        let toml_str = include_str!("../../../lang/en.toml");
        let parsed: LanguageFile =
            toml::from_str(toml_str).expect("Failed to parse embedded en.toml");
        parsed.translations
    });

    map.get(key).cloned().unwrap_or_else(|| key.to_string())
}

/// Translates a key using the active language translation map.
/// Falls back to the centralized English translation if the key is not found.
pub fn t(key: &str) -> String {
    if let Some(lock) = CURRENT_TRANSLATIONS.get() {
        if let Ok(reader) = lock.read() {
            if let Some(val) = reader.get(key) {
                return val.clone();
            }
        }
    }
    get_default_english_translation(key)
}
