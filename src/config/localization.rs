use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageFile {
    pub language_name: String,
    pub translations: HashMap<String, String>,
}

static CURRENT_TRANSLATIONS: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();

/// Discovers all JSON language files in both the configuration directory and the local project root,
/// including the built-in "English" option.
pub fn discover_languages() -> Vec<(String, PathBuf)> {
    let mut langs = Vec::new();
    langs.push(("English".to_string(), PathBuf::new()));

    // 1. Scan the project's root folder 'lang'
    let project_lang_dir = PathBuf::from("lang");
    if project_lang_dir.exists() {
        for (name, path) in discover_languages_in_dir(&project_lang_dir) {
            if !langs.iter().any(|(n, _)| n == &name) {
                langs.push((name, path));
            }
        }
    }

    // 2. Scan the user's config directory 'lang'
    let config_lang_dir = crate::config::paths::get_config_dir().join("lang");
    if config_lang_dir.exists() && config_lang_dir != project_lang_dir {
        for (name, path) in discover_languages_in_dir(&config_lang_dir) {
            // Avoid duplicate entries if they are in both paths
            if !langs.iter().any(|(n, _)| n == &name) {
                langs.push((name, path));
            }
        }
    }

    // 3. Scan the system share directory 'lang' (Linux fallbacks)
    if let Some(share_dir) = crate::config::paths::get_system_share_dir() {
        let share_lang_dir = share_dir.join("lang");
        if share_lang_dir.exists() && share_lang_dir != project_lang_dir && share_lang_dir != config_lang_dir {
            for (name, path) in discover_languages_in_dir(&share_lang_dir) {
                // Avoid duplicate entries
                if !langs.iter().any(|(n, _)| n == &name) {
                    langs.push((name, path));
                }
            }
        }
    }

    // Sort for UI presentation consistency
    langs.sort_by(|a, b| a.0.cmp(&b.0));
    langs
}

/// Helper function to discover language files in a specific directory (makes it testable).
pub fn discover_languages_in_dir(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut langs = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(lang_file) = serde_json::from_str::<LanguageFile>(&content) {
                        langs.push((lang_file.language_name, path));
                    }
                }
            }
        }
    }
    langs.sort_by(|a, b| a.0.cmp(&b.0));
    langs
}

pub mod en;

/// Returns the centralized default English translation for a key.
pub fn get_default_english_translation(key: &str) -> String {
    en::get_default_english_translation(key)
}

/// Loads a language by its full name into the global active translation map.
/// Falls back to empty map (so t() falls back to English) if not found or if English is requested.
pub fn load_language(language_name: &str) {
    if language_name == "English" {
        if let Some(lock) = CURRENT_TRANSLATIONS.get() {
            if let Ok(mut writer) = lock.write() {
                *writer = HashMap::new();
            }
        } else {
            let _ = CURRENT_TRANSLATIONS.set(RwLock::new(HashMap::new()));
        }
        return;
    }

    let langs = discover_languages();

    // Find the file path for the given language name
    let path_opt = langs
        .iter()
        .find(|(name, _)| name == language_name)
        .map(|(_, path)| path.clone());

    let translations = if let Some(path) = path_opt {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(lang_file) = serde_json::from_str::<LanguageFile>(&content) {
                lang_file.translations
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_discovery_and_translation() {
        let dir = tempdir().unwrap();
        let path = dir.path();

        // Write a test JSON file
        let test_lang = LanguageFile {
            language_name: "TestLang".to_string(),
            translations: [("tab_system", "TestSystem")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        };
        let serialized = serde_json::to_string(&test_lang).unwrap();
        let file_path = path.join("test.json");
        std::fs::write(&file_path, serialized).unwrap();

        // Test discover_languages_in_dir
        let discovered = discover_languages_in_dir(path);
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].0, "TestLang");

        // Load the language dynamically by simulating translation loading
        let mut test_translations = HashMap::new();
        test_translations.insert("tab_system".to_string(), "TestSystem".to_string());

        if let Some(lock) = CURRENT_TRANSLATIONS.get() {
            if let Ok(mut writer) = lock.write() {
                *writer = test_translations;
            }
        } else {
            let _ = CURRENT_TRANSLATIONS.set(RwLock::new(test_translations));
        }

        assert_eq!(t("tab_system"), "TestSystem");
        assert_eq!(t("tab_panel"), "&Panel"); // fallback to central English
    }
}
