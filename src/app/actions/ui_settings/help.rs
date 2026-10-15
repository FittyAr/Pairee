use crate::app::state::AppState;
use crate::app::state::popup::About;
use crate::app::state::popup::Help;
use crate::config::localization::{get_active_language_code, t};
use std::path::{Path, PathBuf};

pub fn open_about(state: &mut AppState) {
    state.dialogs.replace(About { scroll_y: 0 });
}

pub async fn open_help(state: &mut AppState) {
    let docs = help_dir().map(|dir| app_docs(&dir)).unwrap_or_default();
    let plugin_docs = plugin_docs().await;
    let first_content = docs
        .first()
        .and_then(|(_, path)| std::fs::read_to_string(path).ok());

    state.dialogs.replace(Help {
        mode: 0,
        docs,
        plugin_docs,
        active_tab: 0,
        cursor_idx: 0,
        scroll_y: 0,
        active_content: first_content,
    });
}

/// The `help/` folders to try, best first: the config folder, the system
/// share folder, then the executable's folder and its parent (a portable
/// install or a build tree); never further up, where an unrelated `help/`
/// (such as `~/help`) could be picked.
fn help_candidates(
    config_dir: &Path,
    share_dir: Option<&Path>,
    exe: Option<&Path>,
) -> Vec<PathBuf> {
    let exe_dirs = exe
        .and_then(Path::parent)
        .into_iter()
        .flat_map(|dir| std::iter::once(dir).chain(dir.parent()));
    std::iter::once(config_dir)
        .chain(share_dir)
        .chain(exe_dirs)
        .map(|dir| dir.join("help"))
        .collect()
}

/// `help/` next to the sources (development builds) or the first of
/// [`help_candidates`] that exists.
fn resolve_help_root() -> Option<PathBuf> {
    let is_dir = |p: &Path| p.is_dir();
    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(manifest_dir).join("help");
        if is_dir(&manifest_path) {
            return Some(manifest_path);
        }
    }
    let exe = std::env::current_exe().ok();
    help_candidates(
        &crate::config::paths::get_config_dir(),
        crate::config::paths::get_system_share_dir().as_deref(),
        exe.as_deref(),
    )
    .into_iter()
    .find(|candidate| is_dir(candidate))
}

/// Help folder of the active language, falling back to English.
fn help_dir() -> Option<PathBuf> {
    let lang_code = get_active_language_code();
    let localized = resolve_help_root().map(|r| r.join(&lang_code));
    if localized.as_ref().is_some_and(|d| d.exists()) {
        localized
    } else {
        resolve_help_root().map(|r| r.join("en"))
    }
}

/// `(title, path)` of every Markdown page in `dir`, sorted by file name.
fn app_docs(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| ext.to_lowercase() == "md")
        })
        .collect();
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    files
        .into_iter()
        .filter_map(|path| {
            let stem = path.file_stem().and_then(|s| s.to_str())?;
            Some((page_title(stem), path))
        })
        .collect()
}

/// Localized `help_title_<stem>`, or the stem in title case.
fn page_title(stem: &str) -> String {
    let translation_key = format!("help_title_{}", stem);
    let title = t(&translation_key);
    if title != translation_key {
        return title;
    }
    stem.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

/// `(plugin name, page)` for loaded plugins that ship help in the active
/// language or in their default language.
async fn plugin_docs() -> Vec<(String, PathBuf)> {
    let mut plugin_docs = Vec::new();
    for p in crate::plugin::registry::get_loaded_plugins().await {
        let help_dir = p.path.join("help");
        if !(help_dir.exists() && help_dir.is_dir()) {
            continue;
        }
        let mut help_path = help_dir.join(format!("{}.md", get_active_language_code()));
        if !help_path.exists() {
            let default_lang = p.manifest.default_language();
            help_path = help_dir.join(format!("{}.md", default_lang));
        }
        if help_path.exists() && help_path.is_file() {
            plugin_docs.push((p.manifest.name.clone(), help_path));
        }
    }
    plugin_docs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_candidates_stop_at_the_executables_parent() {
        let found = help_candidates(
            Path::new("cfg"),
            Some(Path::new("share")),
            Some(Path::new("app/bin/pairee")),
        );
        let expected: Vec<PathBuf> = ["cfg", "share", "app/bin", "app"]
            .iter()
            .map(|d| Path::new(d).join("help"))
            .collect();
        assert_eq!(found, expected, "no help folder further up");
    }
}
