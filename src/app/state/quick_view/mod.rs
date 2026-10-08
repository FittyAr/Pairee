//! Quick view (passive panel preview).
//!
//! Cursor movement only *schedules* a load: after [`QUICK_VIEW_DEBOUNCE`]
//! without further movement, the preview is read by a background job
//! (`app::jobs`) with a size cap, and results are cached by
//! `(path, mtime, size)` so revisiting a file is instant.

pub mod cache;
pub mod load;
mod pdf;

#[cfg(test)]
mod tests;

use super::AppState;
use super::popup::{PopupType, QuickViewDialog};
use crate::app::jobs::JobSlot;
use crate::config::localization::t;
use crate::fs::FileEntry;
use cache::{PreviewCache, PreviewKey};
use load::{QUICK_VIEW_MAX_BYTES, QuickViewPreview};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Quiet period after the last cursor move before a preview is loaded.
pub const QUICK_VIEW_DEBOUNCE: Duration = Duration::from_millis(120);

/// Background loading state of the quick-view panel.
#[derive(Debug, Default)]
pub struct QuickViewState {
    job: JobSlot<(PreviewKey, Arc<QuickViewPreview>)>,
    /// Requested preview and when it was requested (debounce).
    pending: Option<(PreviewKey, Instant)>,
    cache: PreviewCache,
}

impl AppState {
    /// Points the quick view at the hovered/selected entry. Cheap: cached
    /// previews are shown at once, others are loaded by
    /// [`AppState::poll_quick_view`] once the cursor settles.
    pub fn update_quick_view_images(&mut self, allow_image: bool) {
        if !self.panels.quick_view_active {
            return;
        }
        let Some(entry) = self.quick_view_target() else {
            self.quick_view.pending = None;
            self.dialogs.clear();
            return;
        };
        emit_hover(&entry);
        if let Some(PopupType::QuickViewPanel(qv)) = self.dialogs.top()
            && qv.path == entry.path
        {
            return;
        }
        let key = PreviewKey {
            path: entry.path.clone(),
            modified: entry.modified,
            size: entry.size,
            allow_image,
        };
        run_plugin_previewers(&entry.path);
        match self.quick_view.cache.get(&key) {
            Some(preview) => {
                self.quick_view.pending = None;
                self.quick_view.job.cancel();
                self.show_quick_view(entry.path, preview.content.clone(), preview.image.clone());
            }
            None => {
                self.show_quick_view(entry.path, vec![t("quickview_loading")], None);
                self.quick_view.pending = Some((key, Instant::now()));
            }
        }
    }

    /// Starts debounced loads and applies finished ones. Returns `true` when
    /// the preview changed.
    pub fn poll_quick_view(&mut self) -> bool {
        let mut changed = false;
        if let Some((key, preview)) = self.quick_view.job.poll() {
            self.quick_view
                .cache
                .insert(key.clone(), Arc::clone(&preview));
            changed = show_preview(self, &key, &preview);
        }
        let due = self
            .quick_view
            .pending
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() >= QUICK_VIEW_DEBOUNCE);
        if due && let Some((key, _)) = self.quick_view.pending.take() {
            self.quick_view.job.start(move |_| {
                let preview = load::load_preview(&key.path, key.allow_image, QUICK_VIEW_MAX_BYTES);
                (key, Arc::new(preview))
            });
            // Inline execution (no runtime) may already be done.
            changed |= self.poll_quick_view();
        }
        if changed {
            self.mark_ui_dirty();
        }
        changed
    }

    /// First selected entry, else the entry under the cursor.
    fn quick_view_target(&self) -> Option<FileEntry> {
        let active = self.get_active_panel();
        match active.selection_order.first() {
            Some(path) => active.entries.iter().find(|e| &e.path == path).cloned(),
            None => active.entries.get(active.cursor_index).cloned(),
        }
    }

    fn show_quick_view(
        &mut self,
        path: std::path::PathBuf,
        content: Vec<String>,
        image_data: Option<Arc<image::DynamicImage>>,
    ) {
        self.dialogs
            .replace(PopupType::QuickViewPanel(Box::new(QuickViewDialog {
                path,
                content,
                scroll: 0,
                image_data,
                plugin_widget: None,
            })));
    }
}

/// Shows `preview` if the quick view still targets `key.path`.
fn show_preview(state: &mut AppState, key: &PreviewKey, preview: &QuickViewPreview) -> bool {
    let still_wanted = match state.dialogs.top() {
        Some(PopupType::QuickViewPanel(qv)) => qv.path == key.path && qv.plugin_widget.is_none(),
        None => true,
        _ => false,
    };
    if still_wanted && state.panels.quick_view_active {
        state.show_quick_view(
            key.path.clone(),
            preview.content.clone(),
            preview.image.clone(),
        );
    }
    still_wanted
}

/// Fires the plugin `on_hover` hook (no-op outside a Tokio runtime).
fn emit_hover(entry: &FileEntry) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let payload = serde_json::json!({
        "path": entry.path.to_string_lossy(),
        "is_dir": entry.is_dir,
        "size": entry.size,
    });
    handle.spawn(async move {
        crate::plugin::hooks::emit_event("on_hover", payload).await;
    });
}

/// Lets plugin previewers replace the built-in preview (async, best effort).
fn run_plugin_previewers(path: &std::path::Path) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let path = path.to_path_buf();
    handle.spawn(async move {
        for plugin in crate::plugin::registry::get_loaded_plugins().await {
            let job = crate::plugin::registry::PreviewJob {
                file_path: path.clone(),
                area_width: 80,
                area_height: 25,
                skip: 0,
            };
            if let Some(widget) =
                crate::plugin::registry::run_previewer(&plugin.manifest.name, job).await
            {
                let req = crate::plugin::manager::PluginRequest::UpdatePluginWidget {
                    path: path.clone(),
                    widget,
                };
                let _ = crate::plugin::PluginManager::get_sender().send(req).await;
                return;
            }
        }
    });
}
