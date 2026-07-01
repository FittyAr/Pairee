use super::types::LanguageFile;
use std::fs;
use std::path::{Path, PathBuf};

/// Discovers all TOML language files in both the configuration directory and the local project root,
/// including the built-in "English" and "Español" options.
pub fn discover_languages() -> Vec<(String, PathBuf)> {
    let mut langs = Vec::new();

    // 1. Scan the project's root folder 'lang'
    let mut project_lang_dir = PathBuf::from("lang");
    if !project_lang_dir.exists() {
        if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
            let manifest_path = PathBuf::from(manifest_dir).join("lang");
            if manifest_path.exists() {
                project_lang_dir = manifest_path;
            }
        }
    }
    if !project_lang_dir.exists() {
        if let Ok(exe) = std::env::current_exe() {
            let mut current = exe.parent();
            while let Some(dir) = current {
                let candidate = dir.join("lang");
                if candidate.exists() {
                    project_lang_dir = candidate;
                    break;
                }
                current = dir.parent();
            }
        }
    }

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
        if share_lang_dir.exists()
            && share_lang_dir != project_lang_dir
            && share_lang_dir != config_lang_dir
        {
            for (name, path) in discover_languages_in_dir(&share_lang_dir) {
                // Avoid duplicate entries
                if !langs.iter().any(|(n, _)| n == &name) {
                    langs.push((name, path));
                }
            }
        }
    }

    // Ensure built-in languages are present
    if !langs.iter().any(|(n, _)| n == "English") {
        langs.push(("English".to_string(), PathBuf::new()));
    }
    if !langs.iter().any(|(n, _)| n == "Español") {
        langs.push(("Español".to_string(), PathBuf::new()));
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
            if path.extension().map_or(false, |ext| ext == "toml") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(lang_file) = toml::from_str::<LanguageFile>(&content) {
                        langs.push((lang_file.language_name, path));
                    }
                }
            }
        }
    }
    langs.sort_by(|a, b| a.0.cmp(&b.0));
    langs
}
