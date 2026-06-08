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

/// Discovers all JSON language files in the default configuration directory.
pub fn discover_languages() -> Vec<(String, PathBuf)> {
    let lang_dir = crate::config::paths::get_config_dir().join("lang");
    discover_languages_in_dir(&lang_dir)
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
    // Sort for UI presentation consistency
    langs.sort_by(|a, b| a.0.cmp(&b.0));
    langs
}

/// Initializes default English and Spanish translation files if they do not exist.
pub fn init_default_languages() -> anyhow::Result<()> {
    let lang_dir = crate::config::paths::get_config_dir().join("lang");
    init_default_languages_in_dir(&lang_dir)
}

/// Helper function to initialize default languages in a specific directory (makes it testable).
pub fn init_default_languages_in_dir(dir: &Path) -> anyhow::Result<()> {
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }

    let en_path = dir.join("en.json");
    if !en_path.exists() {
        let en_content = LanguageFile {
            language_name: "English".to_string(),
            translations: [
                ("tab_system", "System"),
                ("tab_panel", "Panel"),
                ("tab_interface", "Interface"),
                ("tab_confirmations", "Confirmations"),
                ("tab_plugins", "Language & Plugins"),
                ("tab_editor", "Editor/Viewer"),
                ("tab_colors", "Colors"),
                ("lang_label", "Main language"),
                ("plugins_config", "Plugins configuration"),
                ("plugins_manager_settings", "Plugins manager settings"),
                ("oem_support", "OEM plugins support"),
                ("scan_symlinks", "Scan symbolic links"),
                ("plugin_selection", "Plugin selection"),
                ("file_processing", "File processing"),
                ("show_std_association", "Show standard association"),
                ("even_if_one", "Even if only one plugin"),
                ("search_results", "Search results (SetFindList)"),
                ("prefix_processing", "Prefix processing"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        };
        let serialized = serde_json::to_string_pretty(&en_content)?;
        fs::write(&en_path, serialized)?;
    }

    let es_path = dir.join("es.json");
    if !es_path.exists() {
        let es_content = LanguageFile {
            language_name: "Español".to_string(),
            translations: [
                ("tab_system", "Sistema"),
                ("tab_panel", "Panel"),
                ("tab_interface", "Interfaz"),
                ("tab_confirmations", "Confirmaciones"),
                ("tab_plugins", "Idioma y Plugins"),
                ("tab_editor", "Editor/Visor"),
                ("tab_colors", "Colores"),
                ("lang_label", "Idioma principal"),
                ("plugins_config", "Configuración de plugins"),
                (
                    "plugins_manager_settings",
                    "Configuración del gestor de plugins",
                ),
                ("oem_support", "Soporte de plugins OEM"),
                ("scan_symlinks", "Escanear enlaces simbólicos"),
                ("plugin_selection", "Selección de plugins"),
                ("file_processing", "Procesamiento de archivos"),
                ("show_std_association", "Mostrar asociación estándar"),
                ("even_if_one", "Incluso si solo se encuentra un plugin"),
                ("search_results", "Resultados de búsqueda (SetFindList)"),
                ("prefix_processing", "Procesamiento de prefijos"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        };
        let serialized = serde_json::to_string_pretty(&es_content)?;
        fs::write(&es_path, serialized)?;
    }

    Ok(())
}

/// Loads a language by its full name into the global active translation map.
/// Falls back to English if the language cannot be found or loaded.
pub fn load_language(language_name: &str) {
    let lang_dir = crate::config::paths::get_config_dir().join("lang");
    load_language_from_dir(language_name, &lang_dir);
}

/// Helper function to load a language from a specific directory (makes it testable).
pub fn load_language_from_dir(language_name: &str, dir: &Path) {
    let langs = discover_languages_in_dir(dir);

    // Find the file path for the given language name
    let path_opt = langs
        .iter()
        .find(|(name, _)| name == language_name)
        .map(|(_, path)| path.clone());

    let mut translations = if let Some(path) = path_opt {
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

    // Fallback: If translations are empty and we are not loading English, load English as a fallback.
    if translations.is_empty() && language_name != "English" {
        if let Some(en_path) = langs
            .iter()
            .find(|(name, _)| name == "English")
            .map(|(_, path)| path.clone())
        {
            if let Ok(content) = fs::read_to_string(&en_path) {
                if let Ok(lang_file) = serde_json::from_str::<LanguageFile>(&content) {
                    translations = lang_file.translations;
                }
            }
        }
    }

    if let Some(lock) = CURRENT_TRANSLATIONS.get() {
        if let Ok(mut writer) = lock.write() {
            *writer = translations;
        }
    } else {
        let _ = CURRENT_TRANSLATIONS.set(RwLock::new(translations));
    }
}

/// Translates a key using the active language translation map.
/// Returns the `default` value if the key is not found.
pub fn t(key: &str, default: &str) -> String {
    if let Some(lock) = CURRENT_TRANSLATIONS.get() {
        if let Ok(reader) = lock.read() {
            if let Some(val) = reader.get(key) {
                return val.clone();
            }
        }
    }
    default.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_init_and_discovery() {
        let dir = tempdir().unwrap();
        let path = dir.path();

        // 1. Initialize languages
        init_default_languages_in_dir(path).unwrap();

        // 2. Discover languages
        let discovered = discover_languages_in_dir(path);
        assert_eq!(discovered.len(), 2);

        let names: Vec<String> = discovered.iter().map(|(name, _)| name.clone()).collect();
        assert!(names.contains(&"English".to_string()));
        assert!(names.contains(&"Español".to_string()));
    }

    #[test]
    fn test_load_and_translate() {
        let dir = tempdir().unwrap();
        let path = dir.path();

        init_default_languages_in_dir(path).unwrap();

        // Load English and translate
        load_language_from_dir("English", path);
        assert_eq!(t("tab_system", "Fallback"), "System");
        assert_eq!(t("nonexistent_key", "Fallback"), "Fallback");

        // Load Spanish and translate
        load_language_from_dir("Español", path);
        assert_eq!(t("tab_system", "Fallback"), "Sistema");
        assert_eq!(t("nonexistent_key", "Fallback"), "Fallback");
    }
}
