use crate::app::state::{AppState, PopupType};

pub fn open_about(state: &mut AppState) {
    state.dialogs.replace(PopupType::About { scroll_y: 0 });
}

pub async fn open_help(state: &mut AppState) {
    let mut docs = Vec::new();

    let resolve_help_dir = || -> Option<std::path::PathBuf> {
        // Try CARGO_MANIFEST_DIR first
        if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
            let manifest_path = std::path::PathBuf::from(manifest_dir).join("help");
            if manifest_path.exists() && manifest_path.is_dir() {
                return Some(manifest_path);
            }
        }
        // Try current executable parents
        if let Ok(exe) = std::env::current_exe() {
            let mut current = exe.parent();
            while let Some(dir) = current {
                let candidate = dir.join("help");
                if candidate.exists() && candidate.is_dir() {
                    return Some(candidate);
                }
                current = dir.parent();
            }
        }
        // Try config dir
        let config_path = crate::config::paths::get_config_dir().join("help");
        if config_path.exists() && config_path.is_dir() {
            return Some(config_path);
        }
        // Try system share dir
        if let Some(share_dir) = crate::config::paths::get_system_share_dir() {
            let share_path = share_dir.join("help");
            if share_path.exists() && share_path.is_dir() {
                return Some(share_path);
            }
        }
        None
    };

    let lang_code = crate::config::localization::get_active_language_code();
    let mut help_dir = resolve_help_dir().map(|r| r.join(&lang_code));
    if help_dir.is_none() || !help_dir.as_ref().unwrap().exists() {
        help_dir = resolve_help_dir().map(|r| r.join("en"));
    }

    if let Some(ref dir_path) = help_dir
        && let Ok(entries) = std::fs::read_dir(dir_path)
    {
        let mut files = Vec::new();
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|e| e.to_str())
                && ext.to_lowercase() == "md"
            {
                files.push(path);
            }
        }
        // Sort files alphabetically by filename
        files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        for path in files {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let translation_key = format!("help_title_{}", stem);
                let title = crate::config::localization::t(&translation_key);
                let display_title = if title == translation_key {
                    stem.split('_')
                        .map(|word| {
                            let mut chars = word.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(first) => {
                                    first.to_uppercase().collect::<String>() + chars.as_str()
                                }
                            }
                        })
                        .collect::<Vec<String>>()
                        .join(" ")
                } else {
                    title
                };
                docs.push((display_title, path));
            }
        }
    }

    let mut plugin_docs = Vec::new();
    let loaded_plugins = crate::plugin::registry::get_loaded_plugins().await;
    for p in loaded_plugins {
        let help_dir = p.path.join("help");
        if help_dir.exists() && help_dir.is_dir() {
            let lang_code = crate::config::localization::get_active_language_code();
            let mut help_path = help_dir.join(format!("{}.md", &lang_code));
            if !help_path.exists() {
                let default_lang = p.manifest.default_language.as_deref().unwrap_or("en");
                help_path = help_dir.join(format!("{}.md", default_lang));
            }
            if help_path.exists() && help_path.is_file() {
                plugin_docs.push((p.manifest.name.clone(), help_path));
            }
        }
    }

    let first_content = if !docs.is_empty() {
        std::fs::read_to_string(&docs[0].1).ok()
    } else {
        None
    };

    state.dialogs.replace(PopupType::Help {
        mode: 0,
        docs,
        plugin_docs,
        active_tab: 0,
        cursor_idx: 0,
        scroll_y: 0,
        active_content: first_content,
    });
}
