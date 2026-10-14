use super::types::LanguageFile;
use std::fs;
use std::path::{Path, PathBuf};

/// Discovers all TOML language files in the project's `lang` folder, the
/// configuration directory and the system share directory (the first one
/// found wins for a name), including the built-in "English" and "Español".
pub fn discover_languages() -> Vec<(String, PathBuf)> {
    let mut langs: Vec<(String, PathBuf)> = Vec::new();
    let mut scanned: Vec<PathBuf> = Vec::new();
    let candidates = [
        Some(project_lang_dir()),
        Some(crate::config::paths::get_config_dir().join("lang")),
        crate::config::paths::get_system_share_dir().map(|dir| dir.join("lang")),
    ];
    for dir in candidates.into_iter().flatten() {
        if !dir.exists() || scanned.contains(&dir) {
            continue;
        }
        for (name, path) in discover_languages_in_dir(&dir) {
            if !langs.iter().any(|(n, _)| n == &name) {
                langs.push((name, path));
            }
        }
        scanned.push(dir);
    }
    // Built-in languages (an empty path: embedded in the binary).
    for builtin in ["English", "Español"] {
        if !langs.iter().any(|(n, _)| n == builtin) {
            langs.push((builtin.to_string(), PathBuf::new()));
        }
    }
    // Sort for UI presentation consistency
    langs.sort_by(|a, b| a.0.cmp(&b.0));
    langs
}

/// The `lang` folder shipped with Pairee: in the working directory, the
/// source tree (development builds) or next to (an ancestor of) the
/// executable.
fn project_lang_dir() -> PathBuf {
    let local = PathBuf::from("lang");
    if local.exists() {
        return local;
    }
    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(manifest_dir).join("lang");
        if manifest_path.exists() {
            return manifest_path;
        }
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.ancestors()
                .skip(1)
                .map(|dir| dir.join("lang"))
                .find(|candidate| candidate.exists())
        })
        .unwrap_or(local)
}

/// Helper function to discover language files in a specific directory (makes it testable).
pub fn discover_languages_in_dir(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut langs = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml")
                && let Ok(content) = fs::read_to_string(&path)
                && let Ok(lang_file) = toml::from_str::<LanguageFile>(&content)
            {
                langs.push((lang_file.language_name, path));
            }
        }
    }
    langs.sort_by(|a, b| a.0.cmp(&b.0));
    langs
}
