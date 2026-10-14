use std::sync::{OnceLock, PoisonError, RwLock};

use super::discovery::discover_languages;
use super::translator::{Dictionary, read_dictionary};
use super::types::{CURRENT_LANGUAGE_CODE, CURRENT_TRANSLATIONS, TRANSLATIONS_SECTION};

/// Loads a language by its full name into the global active translation map.
/// Falls back to an empty map (so `t()` falls back to English) if not found
/// or if English is requested.
pub fn load_language(language_name: &str) {
    let (code, translations) = resolve(language_name);
    replace(&CURRENT_TRANSLATIONS, translations);
    replace(&CURRENT_LANGUAGE_CODE, code);
}

/// The code and dictionary of the language called `language_name`: its
/// discovered file (code = file stem), else the embedded Spanish, else
/// English (an empty dictionary).
fn resolve(language_name: &str) -> (String, Dictionary) {
    if language_name == "English" {
        return ("en".to_string(), Dictionary::new());
    }
    let path = discover_languages()
        .into_iter()
        .find(|(name, _)| name == language_name)
        .map(|(_, path)| path)
        .filter(|path| !path.as_os_str().is_empty());
    match path {
        Some(path) => {
            let code = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map_or_else(|| "en".to_string(), str::to_lowercase);
            let translations =
                read_dictionary(&path, Some(TRANSLATIONS_SECTION)).unwrap_or_default();
            (code, translations)
        }
        None if language_name == "Español" => ("es".to_string(), embedded_spanish()),
        None => ("en".to_string(), Dictionary::new()),
    }
}

/// The Spanish translations built into the binary.
fn embedded_spanish() -> Dictionary {
    static EMBEDDED_ES: OnceLock<Dictionary> = OnceLock::new();
    EMBEDDED_ES
        .get_or_init(|| super::translator::embedded(include_str!("../../../lang/es.toml")))
        .clone()
}

/// Sets the global `cell` to `value`.
fn replace<T>(cell: &OnceLock<RwLock<T>>, value: T) {
    let mut value = Some(value);
    let lock = cell.get_or_init(|| RwLock::new(value.take().expect("value not used yet")));
    if let Some(value) = value {
        *lock.write().unwrap_or_else(PoisonError::into_inner) = value;
    }
}

/// Returns the active language code (e.g. "en", "es"); `"en"` until
/// [`load_language`] ran (it never reads `config.toml` itself, so plugin
/// tasks asking for the language cannot trigger a config reload).
pub fn get_active_language_code() -> String {
    CURRENT_LANGUAGE_CODE
        .get()
        .and_then(|lock| lock.read().ok().map(|code| code.clone()))
        .unwrap_or_else(|| "en".to_string())
}
