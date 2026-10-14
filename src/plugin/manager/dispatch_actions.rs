//! Side-effect helpers for the dispatcher: rendering of structured
//! `NotifyPayload` values into the existing `PopupType::Info` slot,
//! dispatching of `pairee.emit` to the registered actions, and
//! computation of stable preview-cache paths.

use super::emit_policy::may_emit;
use super::request::{NotifyPayload, PluginOrigin};
use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PanelState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crate::keybindings::registry::Bindable;
use crate::plugin::runtime::bindings::fs::normalize_lexical;
use std::path::{Path, PathBuf};

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
    state.dialogs.replace(PopupType::info(body));
    log::info!(
        "Plugin notify [{}]: {} - {} (timeout={:?}s)",
        level,
        payload.title,
        payload.content,
        payload.timeout_secs
    );
}

/// Dispatches a `pairee.emit(action, args)` request from `origin`.
///
/// `cd` and `set_focus` / `focus` take arguments and run here: for `cd`,
/// `args` is a string path or `{ path = ... }`; for focus, `"left"` /
/// `"right"` or `{ side = ... }`. Any other name is a keymap action id
/// (`"copy"`, `"go_to_tab_2"`, `"plugin.other.cmd"`): when the emit policy
/// allows it for `origin`, it is queued and the main loop runs it like a
/// key press (this site cannot run async actions).
pub fn dispatch_emit_action(
    state: &mut AppState,
    context: &AppContext,
    origin: &PluginOrigin,
    name: &str,
    args: &serde_json::Value,
) {
    match name {
        "cd" => match string_arg(args, "path") {
            Some(path) => plugin_cd(state, context, &path),
            None => log::warn!("pairee.emit('cd') requires a string or {{ path = ... }} args"),
        },
        "set_focus" | "focus" => match string_arg(args, "side") {
            Some(side) => set_focus(state, &side),
            None => log::warn!("pairee.emit('{name}') requires a string or {{ side = ... }} args"),
        },
        _ => match <Action as Bindable>::from_id(name) {
            Some(action) if may_emit(origin, action) => state.plugins.emitted_actions.push(action),
            Some(_) => {
                log::warn!(
                    "pairee.emit('{name}') refused: untrusted plugin '{}' may not fire it",
                    origin.name
                );
                state.dialogs.replace(PopupType::error(
                    t("plugin_emit_refused")
                        .replacen("{}", &origin.name, 1)
                        .replacen("{}", name, 1),
                ));
            }
            None => log::warn!("pairee.emit('{name}'): no such action"),
        },
    }
}

/// `args` itself when it is a string, else its `key` field.
fn string_arg(args: &serde_json::Value, key: &str) -> Option<String> {
    match args {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(_) => args.get(key)?.as_str().map(str::to_owned),
        _ => None,
    }
}

/// Focuses the `"left"` or `"right"` panel.
pub fn set_focus(state: &mut AppState, side: &str) {
    match side {
        "left" => state.panels.active = ActivePanel::Left,
        "right" => state.panels.active = ActivePanel::Right,
        _ => log::warn!("plugin focus got unknown side {side:?}; expected 'left' or 'right'"),
    }
}

/// Points the active panel at the folder a plugin asked for.
///
/// A relative path is resolved against the panel's folder and the result
/// must be an existing local directory. Remote (SSH) and archive panels
/// are left alone: the request is refused rather than mixing a local path
/// into a remote listing.
pub fn plugin_cd(state: &mut AppState, context: &AppContext, raw: &str) {
    match resolve_cd_target(state.get_active_panel(), raw) {
        Ok(dir) => {
            state.get_active_panel_mut().open_path(dir);
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
        Err(key) => {
            log::warn!("plugin cd to {raw:?} refused ({key})");
            state
                .dialogs
                .replace(PopupType::error(t(key).replacen("{}", raw, 1)));
        }
    }
}

/// The folder `raw` names for `panel`, or the i18n key of the refusal.
fn resolve_cd_target(panel: &PanelState, raw: &str) -> Result<PathBuf, &'static str> {
    if !panel.source.is_local() {
        return Err("plugin_cd_not_local");
    }
    let requested = Path::new(raw);
    let joined = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        panel.current_path.join(requested)
    };
    let dir = normalize_lexical(&joined);
    if dir.is_dir() {
        Ok(dir)
    } else {
        Err("plugin_cd_not_found")
    }
}

/// Computes the cache path for `(plugin, file, skip)`: a stable file name
/// under the user's Pairee cache directory, derived (SHA-256, so it is the
/// same across runs and releases) from the plugin name, the file's path,
/// size and modification time, and `skip`. A changed file therefore gets a
/// new cache entry, and two plugins never share (or poison) one.
pub fn compute_file_cache_path(plugin: &str, file_path: &Path, skip: usize) -> Option<PathBuf> {
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
    let (size, mtime) = std::fs::metadata(&canonical).map_or((0, 0), |m| {
        let nanos = m
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        (m.len(), nanos)
    });
    let key = format!(
        "{plugin}\0{}\0{skip}\0{size}\0{mtime}",
        canonical.to_string_lossy()
    );
    let digest = crate::update::downloader::sha256_hex(key.as_bytes());
    Some(preview_cache.join(&digest[..32]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::PopupType;
    use crate::app::state::popup::InfoMessage;

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
            Some(PopupType::Info(InfoMessage(text))) => assert_eq!(text, "Hello: World"),
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
            Some(PopupType::Info(InfoMessage(text))) => assert_eq!(text, "Only"),
            other => panic!("expected Info popup, got {:?}", other),
        }
    }

    #[test]
    fn cd_resolves_relative_paths_and_requires_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let panel = PanelState::new(dir.path().to_path_buf());
        assert_eq!(
            resolve_cd_target(&panel, "sub/../sub").unwrap(),
            dir.path().join("sub")
        );
        assert_eq!(
            resolve_cd_target(&panel, "missing"),
            Err("plugin_cd_not_found")
        );
        let file = dir.path().join("f.txt");
        std::fs::write(&file, "x").unwrap();
        assert_eq!(
            resolve_cd_target(&panel, &file.to_string_lossy()),
            Err("plugin_cd_not_found")
        );
    }

    #[test]
    fn untrusted_emit_of_destructive_action_is_refused() {
        let mut state = fresh_state();
        let context = AppContext::new(crate::config::AppConfig::default());
        let origin = PluginOrigin {
            name: "demo".into(),
            trusted: false,
        };
        let null = serde_json::Value::Null;
        dispatch_emit_action(&mut state, &context, &origin, "delete_permanent", &null);
        assert!(state.plugins.emitted_actions.is_empty());
        assert!(matches!(state.dialogs.top(), Some(PopupType::Error(_))));
        dispatch_emit_action(&mut state, &context, &origin, "move_down", &null);
        assert_eq!(state.plugins.emitted_actions, vec![Action::MoveDown]);
    }

    #[test]
    fn test_compute_file_cache_path_is_stable() {
        let p = std::env::temp_dir().join("pairee_cache_test.txt");
        std::fs::write(&p, "data").unwrap();
        let a = compute_file_cache_path("demo", &p, 0).expect("cache path");
        let b = compute_file_cache_path("demo", &p, 0).expect("cache path");
        assert_eq!(
            a, b,
            "same (plugin, path, skip) must produce same cache path"
        );
        // Different skip must produce a different cache path.
        let c = compute_file_cache_path("demo", &p, 1).expect("cache path");
        assert_ne!(a, c, "different skip must produce a different cache path");
        // Another plugin never shares the entry.
        let other = compute_file_cache_path("other", &p, 0).expect("cache path");
        assert_ne!(a, other);
        // A changed file (size here; mtime too) gets a fresh entry.
        std::fs::write(&p, "different data").unwrap();
        let changed = compute_file_cache_path("demo", &p, 0).expect("cache path");
        assert_ne!(a, changed);
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn test_compute_file_cache_path_returns_dir() {
        let p = std::env::temp_dir();
        let cache = compute_file_cache_path("demo", &p, 0).expect("cache path");
        // The cache file lives under <cache_dir>/preview_cache/, not at
        // the root temp dir.
        let parent = cache.parent().expect("cache has a parent dir");
        assert!(parent.ends_with("preview_cache"));
    }
}
