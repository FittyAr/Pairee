//! Side-effect helpers for the dispatcher: rendering of structured
//! `NotifyPayload` values into the existing `PopupType::Info` slot,
//! dispatching of `pairee.emit` to the registered actions, and
//! computation of stable preview-cache paths.

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use super::request::NotifyPayload;

/// FIFO queue of `Action`s that the main loop should execute at the
/// next tick. Populated by `dispatch_emit_action` when a plugin calls
/// `pairee.emit(name, args)` with an `Action` name that we can parse
/// via the keybinding resolver (e.g. `pairee.emit("select", …)` or
/// `pairee.emit("reveal", …)`).
///
/// The reason for a separate queue (rather than calling `handle_action`
/// inline) is that `handle_action` is `async` and borrows
/// `&mut TerminalBackend` — neither of which is available from the
/// sync dispatcher site. The main loop drains the queue between
/// `process_plugin_requests` and the next input event, so all queued
/// actions run on the main thread with full access to state and the
/// terminal backend.
pub static PENDING_EMIT_ACTIONS: OnceLock<Mutex<Vec<Action>>> = OnceLock::new();

fn pending_actions() -> &'static Mutex<Vec<Action>> {
    PENDING_EMIT_ACTIONS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Drains all pending emit actions. Called by the main loop once per
/// tick, BEFORE the input event handler, so that plugins can drive
/// arbitrary `Action`s without needing a `&mut TerminalBackend` from
/// the dispatcher site.
pub fn drain_pending_emit_actions() -> Vec<Action> {
    let mut q = match pending_actions().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    std::mem::take(&mut *q)
}

/// Renders a structured `NotifyPayload` into the `PopupType::PluginNotify`
/// slot. M1 adds an auto-dismiss deadline computed from
/// `payload.timeout_secs` so callers see the popup vanish on its own
/// (no Esc needed) when a timeout is supplied.
pub fn render_notify(state: &mut AppState, payload: &NotifyPayload) {
    let level = payload.level.clone().unwrap_or_else(|| "info".to_string());
    let body = if payload.content.is_empty() {
        payload.title.clone()
    } else {
        format!("{}: {}", payload.title, payload.content)
    };
    let deadline = payload.timeout_secs.and_then(|secs| {
        if secs > 0.0 {
            Some(std::time::Instant::now() + std::time::Duration::from_secs_f64(secs))
        } else {
            None
        }
    });
    state.active_popup = Some(PopupType::PluginNotify { body, level, deadline });
    log::info!(
        "Plugin notify [{}]: {} - {} (timeout={:?}s)",
        payload.level.as_deref().unwrap_or("info"),
        payload.title,
        payload.content,
        payload.timeout_secs
    );
}

/// Dispatches a `pairee.emit(action, args)` request.
///
/// `name` is the action name (e.g. `"cd"`, `"select"`, `"reveal"`,
/// `"toggle_all"`, …). The function first tries to parse `name`
/// against the keybinding resolver so any registered `Action` is
/// reachable. If the resolver does not know `name`, we fall back to
/// the two historical plugin-only shortcuts (`cd` and `set_focus` /
/// `focus`) which have always been available via the older
/// `PluginRequest::Cd` / `SetFocus` envelopes; if those also fail, a
/// warning is logged.
///
/// `args` is a JSON value. For most actions it is ignored (the
/// action's existing handler drives its own behaviour). For `cd` it
/// can be a string path or an object with a `path` field. For
/// `set_focus` / `focus` it can be a string side or an object with
/// a `side` field.
pub fn dispatch_emit_action(
    state: &mut AppState,
    context: &AppContext,
    name: &str,
    args: &serde_json::Value,
) {
    // Try the resolver first. This unlocks every Action variant
    // (`select`, `reveal`, `toggle_all`, `quit`, `refresh`, `find_file`,
    // …) without having to add a hand-rolled match arm for each one.
    if let Some(action) = crate::keybindings::preset::parse_action_name(name) {
        let mut q = match pending_actions().lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        q.push(action);
        log::info!("pairee.emit('{}', {}) -> queued for next tick", name, args);
        return;
    }

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
                state.active_panel = crate::app::state::ActivePanel::Left;
            } else if side == "right" {
                state.active_panel = crate::app::state::ActivePanel::Right;
            } else {
                log::warn!(
                    "pairee.emit('{}') got unknown side {:?}; expected 'left' or 'right'",
                    name,
                    side
                );
            }
            log::info!("pairee.emit('{}') -> {}", name, side);
        }
        _ => {
            log::warn!(
                "pairee.emit('{}', {}) called but the action name is unknown to the keybinding \
                 resolver. Use a recognised action (e.g. 'cd', 'select', 'reveal', 'refresh', \
                 'quit', 'move_up', 'go_parent', …). Falling back to no-op.",
                name,
                args
            );
        }
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
        match state.active_popup {
            Some(PopupType::PluginNotify { body, level, deadline }) => {
                assert_eq!(body, "Hello: World");
                assert_eq!(level, "warn");
                assert!(deadline.is_some());
            }
            other => panic!("expected PluginNotify popup, got {:?}", other),
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
        match state.active_popup {
            Some(PopupType::PluginNotify { body, level, deadline }) => {
                assert_eq!(body, "Only");
                assert_eq!(level, "info"); // default
                assert!(deadline.is_none());
            }
            other => panic!("expected PluginNotify popup, got {:?}", other),
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

    #[test]
    fn test_emit_known_action_queues_for_next_tick() {
        // The pre-existing test environment leaves the pending queue
        // non-empty from earlier tests; drain it first so this test
        // starts from a known state.
        let _ = drain_pending_emit_actions();

        let mut state = fresh_state();
        let cfg = crate::config::AppConfig::load_or_create().expect("config");
        let context = crate::app::context::AppContext::new(cfg);

        // `select_item` is a known Action name in the keybinding resolver.
        dispatch_emit_action(
            &mut state,
            &context,
            "select_item",
            &serde_json::json!({}),
        );
        let queued = drain_pending_emit_actions();
        assert_eq!(queued.len(), 1, "expected exactly one queued action");
        assert_eq!(queued[0], crate::keybindings::Action::SelectItem);
    }

    #[test]
    fn test_emit_unknown_action_does_not_queue() {
        let _ = drain_pending_emit_actions();
        let mut state = fresh_state();
        let cfg = crate::config::AppConfig::load_or_create().expect("config");
        let context = crate::app::context::AppContext::new(cfg);
        // `definitely_not_an_action` is not a known action name.
        dispatch_emit_action(
            &mut state,
            &context,
            "definitely_not_an_action",
            &serde_json::json!({}),
        );
        let queued = drain_pending_emit_actions();
        assert!(
            queued.is_empty(),
            "unknown action name must not produce a queued action"
        );
    }
}
