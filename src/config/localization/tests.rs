#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::RwLock;
    use tempfile::tempdir;

    use crate::config::localization::discovery::discover_languages_in_dir;
    use crate::config::localization::loader::load_language;
    use crate::config::localization::translator::{get_default_english_translation, t};
    use crate::config::localization::types::{CURRENT_TRANSLATIONS, LanguageFile};

    #[test]
    fn test_discovery_and_translation() {
        let dir = tempdir().unwrap();
        let path = dir.path();

        // Write a test TOML file
        let test_lang = LanguageFile {
            language_name: "TestLang".to_string(),
            translations: [("tab_system", "TestSystem")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        };
        let serialized = toml::to_string(&test_lang).unwrap();
        let file_path = path.join("test.toml");
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

    #[test]
    fn test_embedded_translations() {
        // Test embedded English fallback
        assert_eq!(get_default_english_translation("tab_system"), "&System");
        assert_eq!(get_default_english_translation("tab_panel"), "&Panel");

        // Test default t() behavior when English is loaded
        load_language("English");
        assert_eq!(t("tab_system"), "&System");

        // Test embedded Spanish fallback
        load_language("Español");
        assert_eq!(t("tab_system"), "&Sistema");
        // Check a key that exists only in English to verify fallback
        assert_eq!(t("git_checkout_branch"), "branch");
    }
}
