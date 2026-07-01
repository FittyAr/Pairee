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

/// Returns the active language code (e.g. "en", "es").
pub fn get_active_language_code() -> String {
    if let Some(lock) = CURRENT_LANGUAGE_CODE.get() {
        if let Ok(reader) = lock.read() {
            return reader.clone();
        }
    }
    // Fallback if not initialized (e.g. called early or in tests)
    if let Ok(config) = crate::config::AppConfig::load_or_create() {
        let active_name = &config.settings.language;
        if active_name.eq_ignore_ascii_case("English") {
            return "en".to_string();
        }
        let langs = discover_languages();
        if let Some((_, path)) = langs.iter().find(|(name, _)| name == active_name) {
            if !path.as_os_str().is_empty() {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    return stem.to_lowercase();
                }
            } else if active_name == "Español" {
                return "es".to_string();
            }
        }
    }
    "en".to_string()
}
