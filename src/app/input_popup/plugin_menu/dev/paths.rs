//! Path resolution helpers for the Developer Tools module: which folder
//! is the "active" dev plugin, and where is the packaged copy inside
//! the temp registry clone.

use crate::app::context::AppContext;
use crate::app::state::AppState;
use std::path::{Path, PathBuf};

/// Number of options in the Developer Tools menu (0-8). Indices 6-8 are
/// the "move panel to folder" shortcuts.
pub const DEV_OPT_COUNT: usize = 9;

/// Move the currently active panel to `path` and close the popup so the
/// user lands directly on the file list.
pub fn move_active_panel_to(state: &mut AppState, path: PathBuf, show_hidden: bool) {
    state.get_active_panel_mut().current_path = path;
    state.refresh_both_panels(show_hidden);
    state.active_popup = None;
}

/// Compute the absolute path of the active development plugin, falling
/// back to `plugins_dev_dir` if there is no active plugin or the folder
/// no longer exists.
pub fn dev_plugin_dir(context: &AppContext) -> PathBuf {
    let base = PathBuf::from(&context.config.settings.plugins_dev_dir);
    match &context.config.settings.active_dev_plugin {
        Some(name) if !name.is_empty() => {
            let candidate = if Path::new(name).is_absolute() {
                PathBuf::from(name)
            } else {
                base.join(name)
            };
            if candidate.exists() { candidate } else { base }
        }
        _ => base,
    }
}

/// Compute the absolute path of the packaged plugin inside the temp
/// registry clone (`cache_dir/temp_registry/registry/plugins/...`).
/// Returns `None` if the manifest cannot be parsed.
pub fn packaged_plugin_dir(active_plugin: &str) -> Option<PathBuf> {
    let plugin_path = PathBuf::from(active_plugin);
    let manifest_path = plugin_path.join("manifest.toml");
    let content = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest = crate::plugin::loader::PluginManifest::parse(&content).ok()?;
    let name = manifest.name;
    let author = manifest.author.as_deref().unwrap_or("unknown").trim();
    let author = if author.is_empty() { "unknown" } else { author };
    let first_char = author.chars().next().unwrap_or('u').to_ascii_lowercase();
    let first_char_str = if first_char.is_ascii_alphabetic() {
        first_char.to_string()
    } else {
        "_".to_string()
    };
    Some(
        crate::config::paths::get_cache_dir()
            .join("temp_registry")
            .join("registry")
            .join("plugins")
            .join(first_char_str)
            .join(author)
            .join(&name),
    )
}
