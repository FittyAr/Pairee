//! Plugin previewers in the quick view: `preload`, `peek` and `seek`.
//!
//! When the quick view targets a file, every loaded plugin that defines
//! `preload` prepares it and the first plugin (by name) whose `peek`
//! returns a widget replaces the built-in preview. Then the next
//! [`PRELOAD_AHEAD`] files of the panel are preloaded in the background,
//! so moving the cursor down finds their previews ready. PgUp / PgDn on a
//! plugin preview whose plugin defines `seek` ask it for the new `skip`
//! and peek again. Everything runs in Tokio tasks; results reach the main
//! loop as `PluginRequest::UpdatePluginWidget`.

use crate::app::state::popup::PluginPreview;
use crate::plugin::manager::PluginRequest;
use crate::plugin::registry::{self, PluginInfo, PreviewJob};
use std::path::{Path, PathBuf};

/// Files after the cursor that are preloaded once the preview is shown.
pub const PRELOAD_AHEAD: usize = 3;

/// Size of the preview area: the passive panel without its frame, or
/// 80×25 without a terminal (tests).
fn preview_area() -> (u16, u16) {
    crossterm::terminal::size()
        .map(|(w, h)| ((w / 2).saturating_sub(2).max(1), h.saturating_sub(5).max(1)))
        .unwrap_or((80, 25))
}

fn job(path: &Path, skip: usize) -> PreviewJob {
    let (area_width, area_height) = preview_area();
    PreviewJob {
        file_path: path.to_path_buf(),
        area_width,
        area_height,
        skip,
    }
}

/// The loaded plugins, by name, that can preview.
async fn previewers() -> Vec<PluginInfo> {
    let mut plugins: Vec<PluginInfo> = registry::get_loaded_plugins()
        .await
        .into_iter()
        .filter(|p| p.methods.peek || p.methods.preload)
        .collect();
    plugins.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    plugins
}

/// Peeks `path` with `plugin` at `skip` and hands the widget to the main
/// loop; `false` when the plugin drew nothing.
async fn show(plugin: &PluginInfo, path: &Path, skip: usize) -> bool {
    let name = &plugin.manifest.name;
    let Some(widget) = registry::run_previewer(name, job(path, skip)).await else {
        return false;
    };
    let preview = Box::new(PluginPreview {
        widget,
        plugin: name.clone(),
        skip,
        seekable: plugin.methods.seek,
    });
    let req = PluginRequest::UpdatePluginWidget {
        path: path.to_path_buf(),
        preview,
    };
    let _ = crate::plugin::PluginManager::get_sender().send(req).await;
    true
}

/// Lets plugin previewers replace the built-in preview of `path`, then
/// preloads `ahead` (async, best effort; no-op outside a Tokio runtime).
pub fn run_plugin_previewers(path: &Path, ahead: Vec<PathBuf>) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let path = path.to_path_buf();
    handle.spawn(async move {
        let plugins = previewers().await;
        let mut shown = false;
        for plugin in &plugins {
            if plugin.methods.preload {
                registry::run_preload(&plugin.manifest.name, job(&path, 0)).await;
            }
            if !shown && plugin.methods.peek {
                shown = show(plugin, &path, 0).await;
            }
        }
        for next in &ahead {
            for plugin in plugins.iter().filter(|p| p.methods.preload) {
                registry::run_preload(&plugin.manifest.name, job(next, 0)).await;
            }
        }
    });
}

/// PgUp / PgDn (`units` lines) on a plugin preview of `path` whose plugin
/// defines `seek`: asks it for the new `skip` and peeks again.
pub fn seek_plugin_preview(path: &Path, preview: &PluginPreview, units: i64) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let (path, name, skip) = (path.to_path_buf(), preview.plugin.clone(), preview.skip);
    handle.spawn(async move {
        let Some(plugin) = registry::plugin_info(&name).await else {
            return;
        };
        if let Some(new_skip) = registry::run_seek(&name, job(&path, skip), units).await {
            show(&plugin, &path, new_skip).await;
        }
    });
}
