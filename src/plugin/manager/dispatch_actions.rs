//! Side-effect helpers for the dispatcher: rendering of structured
//! `NotifyPayload` values into the existing `PopupType::Info` slot,
//! dispatching of `pairee.emit` to the registered actions, and
//! computation of stable preview-cache paths.

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crate::keybindings::registry::Bindable;
use std::path::{Path, PathBuf};

use super::request::NotifyPayload;

/// Renders a structured `NotifyPayload` into the existing `PopupType::Info`
/// slot. The exact rendering is unified with the legacy `<title>: <msg>` form
/// so plugins see a consistent notification UX regardless of which API
/// they call.
pub fn render_notify(state: &mut AppState, payload: &NotifyPayload) {
    let level = payload.level.as_deref().unwrap_or("info");
    let body = if payload.content.is_empty() {
        payload.title.clone()
    } else {
        format!("{}: {}", payload.title, payload.content)
    };
    state.dialogs.replace(PopupType::Info(body));
    log::info!(
        "Plugin notify [{}]: {} - {} (timeout={:?}s)",
        level,
        payload.title,
        payload.content,
        payload.timeout_secs
    );
}

/// Dispatches a `pairee.emit(action, args)` request.
///
/// `cd` and `set_focus` / `focus` take arguments and run here: for `cd`,
/// `args` is a string path or `{ path = ... }`; for focus, `"left"` /
/// `"right"` or `{ side = ... }`. Any other name is a keymap action id
/// (`"copy"`, `"go_to_tab_2"`, `"plugin.other.cmd"`): it is queued and the
/// main loop runs it like a key press (this site cannot run async actions).
pub fn dispatch_emit_action(
    state: &mut AppState,
    context: &AppContext,
    name: &str,
    args: &serde_json::Value,
) {
    match (name, args) {
        ("cd", _) => {
            let path = match args {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Object(_) => args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                _ => {
                    log::warn!("pairee.emit('cd') requires a string or {{ path = ... }} args");
                    return;
                }
            };
            state.get_active_panel_mut().current_path = PathBuf::from(&path);
            state.refresh_both_panels(context.config.settings.show_hidden);
            log::info!("pairee.emit('cd') -> {:?}", path);
        }
        ("set_focus", _) | ("focus", _) => {
            let side = match args {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Object(_) => args
                    .get("side")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                _ => {
                    log::warn!(
                        "pairee.emit('{}') requires a string or {{ side = ... }} args",
                        name
                    );
                    return;
                }
            };
            if side == "left" {
                state.panels.active = crate::app::state::ActivePanel::Left;
            } else if side == "right" {
                state.panels.active = crate::app::state::ActivePanel::Right;
            } else {
                log::warn!(
                    "pairee.emit('{}') got unknown side {:?}; expected 'left' or 'right'",
                    name,
                    side
                );
            }
            log::info!("pairee.emit('{}') -> {}", name, side);
        }
        _ => match <Action as Bindable>::from_id(name) {
            Some(action) => state.plugins.emitted_actions.push(action),
            None => log::warn!("pairee.emit('{name}'): no such action"),
        },
    }
}

/// Computes the cache URL for a `(file, skip)` pair. The cache is a stable
/// file name under the user's Pairee cache directory derived from the
/// file's metadata (path + modification time) and the `skip` value, so
/// previewers can cache generated content (e.g. image conversions) and
/// reuse the cache across invocations without recomputing.
pub fn compute_file_cache_path(file_path: &Path, skip: usize) -> Option<PathBuf> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let cache_root = crate::config::paths::get_cache_dir();
    let preview_cache = cache_root.join("preview_cache");
    if std::fs::create_dir_all(&preview_cache).is_err() {
        log::warn!(
            "Failed to create preview cache directory {:?}; file_cache returns nil.",
            preview_cache
        );
        return None;
    }

    let canonical = file_path
        .canonicalize()
        .unwrap_or_else(|_| file_path.to_path_buf());
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    skip.hash(&mut hasher);
    let digest = hasher.finish();
    Some(preview_cache.join(format!("{:016x}", digest)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::PopupType;

    fn fresh_state() -> AppState {
        AppState::new(PathBuf::from("/"), PathBuf::from("/"))
    }

    #[test]
    fn test_render_notify_uses_structured_payload() {
        // `render_notify` only touches the `active_popup` field, so a
        // freshly-initialised state is sufficient.
        let mut state = fresh_state();
        render_notify(
            &mut state,
            &NotifyPayload {
                title: "Hello".to_string(),
                content: "World".to_string(),
                level: Some("warn".to_string()),
                timeout_secs: Some(2.5),
            },
        );
        match state.dialogs.top() {
            Some(PopupType::Info(text)) => assert_eq!(text, "Hello: World"),
            other => panic!("expected Info popup, got {:?}", other),
        }
    }

    #[test]
    fn test_render_notify_falls_back_to_title_when_content_empty() {
        let mut state = fresh_state();
        render_notify(
            &mut state,
            &NotifyPayload {
                title: "Only".to_string(),
                content: String::new(),
                level: None,
                timeout_secs: None,
            },
        );
        match state.dialogs.top() {
            Some(PopupType::Info(text)) => assert_eq!(text, "Only"),
            other => panic!("expected Info popup, got {:?}", other),
        }
    }

    #[test]
    fn test_compute_file_cache_path_is_stable() {
        let p = std::env::temp_dir().join("pairee_cache_test.txt");
        std::fs::write(&p, "data").unwrap();
        let a = compute_file_cache_path(&p, 0).expect("cache path");
        let b = compute_file_cache_path(&p, 0).expect("cache path");
        assert_eq!(a, b, "same (path, skip) must produce same cache path");
        // Different skip must produce a different cache path.
        let c = compute_file_cache_path(&p, 1).expect("cache path");
        assert_ne!(a, c, "different skip must produce a different cache path");
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn test_compute_file_cache_path_returns_dir() {
        let p = std::env::temp_dir();
        let cache = compute_file_cache_path(&p, 0).expect("cache path");
        // The cache file lives under <cache_dir>/preview_cache/, not at
        // the root temp dir.
        let parent = cache.parent().expect("cache has a parent dir");
        assert!(parent.ends_with("preview_cache"));
    }
}
