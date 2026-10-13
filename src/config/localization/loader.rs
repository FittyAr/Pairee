use std::collections::HashMap;
use std::fs;
use std::sync::OnceLock;
use std::sync::RwLock;

use super::discovery::discover_languages;
use super::types::{CURRENT_LANGUAGE_CODE, CURRENT_TRANSLATIONS, LanguageFile};

/// Loads a language by its full name into the global active translation map.
/// Falls back to empty map (so t() falls back to English) if not found or if English is requested.
pub fn load_language(language_name: &str) {
    let mut code = "en".to_string();
    if language_name == "English" {
        if let Some(lock) = CURRENT_TRANSLATIONS.get() {
            if let Ok(mut writer) = lock.write() {
                *writer = HashMap::new();
            }
        } else {
            let _ = CURRENT_TRANSLATIONS.set(RwLock::new(HashMap::new()));
        }
    } else {
        let langs = discover_languages();

        // Find the file path for the given language name
        let path_opt = langs
            .iter()
            .find(|(name, _)| name == language_name)
            .map(|(_, path)| path.clone());

        let translations = if let Some(path) = path_opt {
            if !path.as_os_str().is_empty() {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    code = stem.to_lowercase();
                }
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(lang_file) = toml::from_str::<LanguageFile>(&content) {
                        lang_file.translations
                    } else {
                        HashMap::new()
                    }
                } else {
                    HashMap::new()
                }
            } else if language_name == "Español" {
                // Built-in Spanish fallback
                code = "es".to_string();
                static EMBEDDED_ES_MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
                EMBEDDED_ES_MAP
                    .get_or_init(|| {
                        let toml_str = include_str!("../../../lang/es.toml");
                        let parsed: LanguageFile =
                            toml::from_str(toml_str).expect("Failed to parse embedded es.toml");
                        parsed.translations
                    })
                    .clone()
            } else {
                HashMap::new()
            }
        } else if language_name == "Español" {
            // Built-in Spanish fallback (just in case)
            code = "es".to_string();
            static EMBEDDED_ES_MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
            EMBEDDED_ES_MAP
                .get_or_init(|| {
                    let toml_str = include_str!("../../../lang/es.toml");
                    let parsed: LanguageFile =
                        toml::from_str(toml_str).expect("Failed to parse embedded es.toml");
                    parsed.translations
                })
                .clone()
        } else {
            HashMap::new()
        };

        if let Some(lock) = CURRENT_TRANSLATIONS.get() {
            if let Ok(mut writer) = lock.write() {
                *writer = translations;
            }
        } else {
            let _ = CURRENT_TRANSLATIONS.set(RwLock::new(translations));
        }
    }

    if let Some(lock) = CURRENT_LANGUAGE_CODE.get() {
        if let Ok(mut writer) = lock.write() {
            *writer = code;
        }
    } else {
        let _ = CURRENT_LANGUAGE_CODE.set(RwLock::new(code));
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
