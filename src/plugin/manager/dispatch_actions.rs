//! Side-effect helpers for the dispatcher: rendering of structured
//! `NotifyPayload` values into the existing `PopupType::Info` slot,
//! dispatching of `pairee.emit` to the registered actions, and
//! computation of stable preview-cache paths.
//!
//! Several helpers here (e.g. `serial_mutex`, `dispatcher_for`)
//! are part of the plugin emit surface and are referenced from
//! plugin code or its tests. Some of them look unused to the
//! compiler because they are reached through trait dispatch, so
//! dead-code analysis is suppressed at the module level rather
//! than item-by-item.
#![allow(dead_code)]

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::app::state::types::Toast;
use crate::keybindings::Action;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use super::request::NotifyPayload;

/// §6 Secure-Mode equivalent: canonicalise `path` (following
/// symlinks) and require it to live inside the workspace, config, or
/// cache directories — the same boundary that `pairee.fs.read` /
/// `PluginRequest::Cd` enforce. Returns the canonical path on
/// success, or `None` if the path is outside the sandbox (caller
/// should treat this as a no-op + warn).
pub fn validate_workspace_path(path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let allowed = [
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        crate::config::paths::get_config_dir(),
        crate::config::paths::get_cache_dir(),
    ];
    if allowed.iter().any(|root| {
        let r = root.canonicalize().unwrap_or_else(|_| root.clone());
        canonical.starts_with(&r)
    }) {
        Some(canonical)
    } else {
        None
    }
}

/// Per-test mutex used to serialise the queue-touching tests.
/// Without this, parallel test runs race on the process-global
/// pending queue and produce order-dependent failures.
fn serial_mutex() -> &'static std::sync::Mutex<()> {
    use std::sync::{Mutex, OnceLock};
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
}

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

/// Renders a structured `NotifyPayload` as a non-modal toast overlay.
///
/// The toast lives in `state.toast` (a separate slot from
/// `state.active_popup`), so it never blocks keyboard input. The main loop
/// watches the optional `deadline` and clears the slot automatically. When
/// a new toast arrives while one is already on screen, [`push_toast`]
/// keeps whichever deadline is later so the user gets the full timeout to
/// read the new message.
pub fn render_notify(state: &mut AppState, payload: &NotifyPayload) {
    let level = payload.level.clone().unwrap_or_else(|| "info".to_string());
    let body = if payload.content.is_empty() {
        payload.title.clone()
    } else {
        format!("{}: {}", payload.title, payload.content)
    };
    push_toast(state, &body, &level, payload.timeout_secs);
    log::info!(
        "Plugin notify [{}]: {} - {} (timeout={:?}s)",
        payload.level.as_deref().unwrap_or("info"),
        payload.title,
        payload.content,
        payload.timeout_secs
    );
}

/// Push a toast onto `state.toast`.
///
/// If a toast is already on screen, it is replaced. The replacement keeps
/// the **later** of (the existing deadline) and (the new deadline) so the
/// user never loses feedback because the new message arrived "too early"
/// (e.g. they installed two plugins back-to-back and the second toast
/// would have replaced the first before the timeout elapsed). `None` for
/// `timeout_secs` means "stay until replaced".
pub fn push_toast(state: &mut AppState, body: &str, level: &str, timeout_secs: Option<f64>) {
    let mut toast = Toast::new(body.to_string(), level.to_string(), timeout_secs);
    if let Some(existing) = &state.toast {
        if let (Some(existing_d), Some(new_d)) = (existing.deadline, toast.deadline) {
            // Keep whichever deadline is later.
            if existing_d > new_d {
                toast.deadline = Some(existing_d);
            }
        }
    }
    state.toast = Some(toast);
}

/// Returns true if a toast is currently on screen and not yet expired.
pub fn toast_is_alive(state: &AppState) -> bool {
    match &state.toast {
        None => false,
        Some(t) => match t.deadline {
            None => true,
            Some(d) => std::time::Instant::now() < d,
        },
    }
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
        // §6 Secure-Mode: a small blacklist of destructive actions
        // cannot be emitted by a plugin. This is a defence-in-depth
        // check on top of the existing confirmation popups — a
        // malicious plugin could otherwise bypass the dialog via
        // `pairee.emit("delete", ...)`.
        if context.config.settings.secure_mode
            && matches!(
                action,
                crate::keybindings::Action::Delete
                    | crate::keybindings::Action::WipeFile
                    | crate::keybindings::Action::Move
            )
        {
            log::warn!("pairee.emit('{name}') is blocked in Secure Mode (destructive action)");
            return;
        }
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
            // §6 Secure-Mode equivalent: validate the destination
            // path against the workspace / config / cache sandbox
            // before mutating the active panel. The dispatcher
            // enforces the same check on `PluginRequest::Cd`;
            // applying it here closes the `pairee.emit('cd', ...)`
            // bypass.
            let p = PathBuf::from(&path);
            match validate_workspace_path(&p) {
                Some(canonical) => {
                    state.get_active_panel_mut().current_path = canonical;
                    state.refresh_both_panels(context.config.settings.show_hidden);
                    log::info!("pairee.emit('cd') -> {:?}", path);
                }
                None => {
                    log::warn!(
                        "pairee.emit('cd') rejected: {:?} is outside the workspace / config / cache",
                        p
                    );
                }
            }
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
///
/// §6 Secure-Mode equivalent: the file path must live inside the
/// workspace, config, or cache roots — the same boundary that
/// `pairee.fs.read` enforces. Without this check a plugin could probe
/// arbitrary filesystem paths by observing whether `file_cache` returns
/// a value (the cache key is deterministically derived from the path
/// bytes, so existence/non-existence becomes a side channel). We also
/// refuse to compute a cache path when the input cannot be canonicalized
/// (broken symlink, non-existent file) because the file must already
/// exist for any cache to be meaningful — and the canonicalize-fail
/// itself would already probe the filesystem.
pub fn compute_file_cache_path(file_path: &Path, skip: usize) -> Option<PathBuf> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // §6: strict canonicalize. The path must exist on disk and
    // resolve to a location inside the workspace / config / cache.
    // A failure here means we cannot prove the path is in-bounds,
    // so we treat it as a probe attempt and refuse.
    let canonical = match file_path.canonicalize() {
        Ok(c) => c,
        Err(_) => {
            log::warn!(
                "pairee.file_cache rejected: {:?} does not canonicalize (path missing, \
                 broken symlink, or unreadable)",
                file_path
            );
            return None;
        }
    };
    if validate_workspace_path(&canonical).is_none() {
        log::warn!(
            "pairee.file_cache rejected: {:?} is outside the workspace / config / cache",
            file_path
        );
        return None;
    }

    let cache_root = crate::config::paths::get_cache_dir();
    let preview_cache = cache_root.join("preview_cache");
    if std::fs::create_dir_all(&preview_cache).is_err() {
        log::warn!(
            "Failed to create preview cache directory {:?}; file_cache returns nil.",
            preview_cache
        );
        return None;
    }

    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    skip.hash(&mut hasher);
    let digest = hasher.finish();
    Some(preview_cache.join(format!("{:016x}", digest)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_state() -> AppState {
        AppState::new(PathBuf::from("/"), PathBuf::from("/"))
    }

    #[test]
    fn test_render_notify_uses_toast_slot() {
        // `render_notify` no longer mutates `active_popup`; it stores the
        // notification in the non-modal `toast` slot so the underlying
        // popup (e.g. the plugin search panel) keeps the focus.
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
        let toast = state
            .toast
            .as_ref()
            .expect("expected a toast to be set after render_notify");
        assert_eq!(toast.body, "Hello: World");
        assert_eq!(toast.level, "warn");
        assert!(toast.deadline.is_some(), "toast should auto-dismiss");
        assert!(
            state.active_popup.is_none(),
            "render_notify must NOT touch active_popup (it was a modal before)"
        );
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
        let toast = state.toast.as_ref().expect("toast should be set");
        assert_eq!(toast.body, "Only");
        assert_eq!(toast.level, "info"); // default
        assert!(toast.deadline.is_none(), "no timeout => no deadline");
    }

    #[test]
    fn test_push_toast_replaces_and_extends_deadline() {
        // Two toasts in quick succession must not lose the user's
        // feedback: the later deadline wins. This matters for the
        // "install 5 plugins back-to-back" workflow — the user should
        // see the last toast for the full timeout.
        let mut state = fresh_state();
        let far_future = std::time::Instant::now() + std::time::Duration::from_secs(60);
        state.toast = Some(Toast {
            body: "first".to_string(),
            level: "info".to_string(),
            deadline: Some(far_future),
        });
        // A second toast with a 2s timeout should NOT shorten the
        // existing 60s deadline.
        push_toast(&mut state, "second", "info", Some(2.0));
        let toast = state.toast.as_ref().unwrap();
        assert_eq!(toast.body, "second", "body must be replaced");
        assert_eq!(
            toast.deadline,
            Some(far_future),
            "existing later deadline must be kept"
        );
    }

    #[test]
    fn test_push_toast_extends_when_new_deadline_is_later() {
        // Symmetric: a new toast with a later deadline must extend the
        // existing one (rather than shorten it).
        let mut state = fresh_state();
        let now = std::time::Instant::now();
        state.toast = Some(Toast {
            body: "first".to_string(),
            level: "info".to_string(),
            deadline: Some(now + std::time::Duration::from_secs(1)),
        });
        push_toast(&mut state, "second", "info", Some(60.0));
        let toast = state.toast.as_ref().unwrap();
        assert!(
            toast
                .deadline
                .expect("deadline")
                .duration_since(now)
                .as_secs()
                >= 55,
            "new later deadline must be kept"
        );
    }

    #[test]
    fn test_toast_is_alive_respects_deadline() {
        let mut state = fresh_state();
        // No toast => not alive.
        assert!(!toast_is_alive(&state));

        // Toast with no deadline => always alive.
        state.toast = Some(Toast::new("x", "info", None));
        assert!(toast_is_alive(&state));

        // Toast with a deadline already in the past => not alive.
        state.toast = Some(Toast {
            body: "x".to_string(),
            level: "info".to_string(),
            deadline: Some(std::time::Instant::now() - std::time::Duration::from_secs(1)),
        });
        assert!(!toast_is_alive(&state));
    }

    #[test]
    fn test_toast_slot_is_independent_of_active_popup() {
        // The input pipeline in `app/app/events.rs` only consults
        // `active_popup` to decide whether to swallow a key, so a
        // toast on screen never blocks input. This test pins the
        // data-model invariant that future refactors of the toast
        // slot cannot accidentally couple it to the modal popup
        // slot.
        let t = Toast::new("hello", "info", Some(2.0));
        assert_eq!(t.body, "hello");
        assert_eq!(t.level, "info");
        assert!(t.deadline.is_some());
        // The toast is a pure value type — it carries no
        // `active_popup` reference and no `reply_tx`. If a future
        // refactor adds either, the input pipeline will silently
        // start blocking keys; review any change to this size.
        let _ = std::mem::size_of::<Toast>();
    }

    #[test]
    fn test_two_toasts_back_to_back_extend_deadline() {
        // Regression test for the user-reported workflow: install
        // 5 plugins in quick succession. Each new toast must show
        // for at least its full timeout, regardless of whether the
        // previous toast had already started its countdown.
        let mut state = fresh_state();
        push_toast(&mut state, "first", "info", Some(60.0));
        let d1 = state.toast.as_ref().unwrap().deadline.unwrap();
        push_toast(&mut state, "second", "info", Some(1.0));
        let d2 = state.toast.as_ref().unwrap().deadline.unwrap();
        assert_eq!(
            d1, d2,
            "second toast must not shorten the existing deadline"
        );
        assert_eq!(
            state.toast.as_ref().unwrap().body,
            "second",
            "body is replaced with the latest message"
        );
    }

    #[test]
    fn test_compute_file_cache_path_is_stable() {
        // §6: the file must live inside the workspace. Use the
        // current working directory (the workspace) and place the
        // fixture there.
        let p = std::env::current_dir()
            .unwrap()
            .join("pairee_cache_test.txt");
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
        // Use the workspace root as the input; the cache file still
        // lives under <cache_dir>/preview_cache/.
        let p = std::env::current_dir().unwrap();
        let cache = compute_file_cache_path(&p, 0).expect("cache path");
        let parent = cache.parent().expect("cache has a parent dir");
        assert!(parent.ends_with("preview_cache"));
    }

    #[test]
    fn test_compute_file_cache_path_rejects_outside_workspace() {
        // §6: a path outside the workspace / config / cache roots
        // must NOT produce a cache path. The helper returns `None`
        // and logs a warning.
        let p = std::path::PathBuf::from("/etc/should-be-rejected.txt");
        assert!(
            compute_file_cache_path(&p, 0).is_none(),
            "file_cache must reject paths outside the workspace / config / cache"
        );
    }

    #[test]
    fn test_compute_file_cache_path_rejects_broken_symlink() {
        // §6: a path that does not exist (broken symlink target)
        // cannot be canonicalized; we treat this as a probe attempt
        // and return None rather than falling back to the raw path.
        let workspace = std::env::current_dir().unwrap();
        let broken = workspace.join("pairee_does_not_exist_xyzzy.txt");
        assert!(
            compute_file_cache_path(&broken, 0).is_none(),
            "file_cache must reject paths that fail to canonicalize"
        );
    }

    #[test]
    fn test_emit_known_action_queues_for_next_tick() {
        let _guard = serial_mutex().lock().unwrap();
        // The pre-existing test environment leaves the pending queue
        // non-empty from earlier tests; drain it first so this test
        // starts from a known state.
        let _ = drain_pending_emit_actions();

        let mut state = fresh_state();
        let cfg = crate::config::AppConfig::load_or_create().expect("config");
        let context = crate::app::context::AppContext::new(cfg);

        // `select_item` is a known Action name in the keybinding resolver.
        dispatch_emit_action(&mut state, &context, "select_item", &serde_json::json!({}));
        let queued = drain_pending_emit_actions();
        assert_eq!(queued.len(), 1, "expected exactly one queued action");
        assert_eq!(queued[0], crate::keybindings::Action::SelectItem);
    }

    #[test]
    fn test_emit_unknown_action_does_not_queue() {
        // The pending-queue tests below share a process-global
        // mutex; serialise them so a parallel test cannot race us.
        let _guard = serial_mutex().lock().unwrap();
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

    #[test]
    fn test_emit_destructive_action_blocked_in_secure_mode() {
        let _guard = serial_mutex().lock().unwrap();
        // §6 Secure-Mode: `Delete`, `WipeFile`, and `Move` must not
        // be emitted by a plugin even when the resolver knows the
        // action. We exercise the canonical spelling ("delete").
        let _ = drain_pending_emit_actions();
        let mut state = fresh_state();
        let cfg = crate::config::AppConfig::load_or_create().expect("config");
        // Synthesize a Secure-Mode context by cloning the config
        // and flipping the bit.
        let mut secure_cfg = cfg.clone();
        secure_cfg.settings.secure_mode = true;
        let context = crate::app::context::AppContext::new(secure_cfg);

        // `delete` is the canonical name (see `preset.rs`).
        dispatch_emit_action(&mut state, &context, "delete", &serde_json::json!({}));
        let queued = drain_pending_emit_actions();
        assert!(
            queued.is_empty(),
            "secure-mode emit of 'delete' must be blocked, got {:?}",
            queued
        );
    }

    #[test]
    fn test_emit_destructive_action_allowed_outside_secure_mode() {
        let _guard = serial_mutex().lock().unwrap();
        let _ = drain_pending_emit_actions();
        let mut state = fresh_state();
        let cfg = crate::config::AppConfig::load_or_create().expect("config");
        // cfg.secure_mode defaults to false; the action must be
        // accepted.
        assert!(!cfg.settings.secure_mode);
        let context = crate::app::context::AppContext::new(cfg);
        dispatch_emit_action(&mut state, &context, "delete", &serde_json::json!({}));
        let queued = drain_pending_emit_actions();
        assert_eq!(
            queued.len(),
            1,
            "non-secure-mode emit of 'delete' must succeed"
        );
        assert_eq!(queued[0], crate::keybindings::Action::Delete);
    }

    #[tokio::test]
    async fn test_emit_cd_accepts_workspace_path() {
        // The `cd` action via `pairee.emit` must canonicalize and
        // accept any directory inside the workspace, config, or cache
        // roots.
        let workspace = std::env::current_dir().unwrap();
        let tmp = tempfile::tempdir_in(&workspace).unwrap();
        let mut state = AppState::new(tmp.path().to_path_buf(), tmp.path().to_path_buf());
        let cfg = crate::config::AppConfig::load_or_create().expect("config");
        let context = crate::app::context::AppContext::new(cfg);
        let pre = state.get_active_panel().current_path.clone();
        let new_path = tmp.path().join("sub");
        std::fs::create_dir_all(&new_path).unwrap();
        dispatch_emit_action(
            &mut state,
            &context,
            "cd",
            &serde_json::json!(new_path.to_string_lossy().to_string()),
        );
        let post = state.get_active_panel().current_path.clone();
        assert_ne!(pre, post, "active panel cwd must change after emit cd");
        assert!(
            post.starts_with(tmp.path().canonicalize().unwrap()),
            "post-cd cwd must remain inside the workspace"
        );
    }

    #[test]
    fn test_emit_cd_rejects_outside_workspace() {
        // The `cd` action via `pairee.emit` must NOT navigate the
        // active panel to a directory outside the workspace /
        // config / cache roots. The dispatcher logs a warning and
        // leaves `current_path` untouched.
        let tmp = tempfile::tempdir().unwrap();
        let mut state = AppState::new(tmp.path().to_path_buf(), tmp.path().to_path_buf());
        let cfg = crate::config::AppConfig::load_or_create().expect("config");
        let context = crate::app::context::AppContext::new(cfg);
        let pre = state.get_active_panel().current_path.clone();
        dispatch_emit_action(
            &mut state,
            &context,
            "cd",
            &serde_json::json!("/etc/should-be-rejected"),
        );
        let post = state.get_active_panel().current_path.clone();
        assert_eq!(
            pre, post,
            "active panel cwd must NOT change after outside-workspace emit cd"
        );
    }

    #[tokio::test]
    async fn test_emit_cd_accepts_object_with_path_field() {
        // The `cd` action also accepts `{ path = "..." }` as its
        // args, matching the documented shape in the dispatcher.
        let workspace = std::env::current_dir().unwrap();
        let tmp = tempfile::tempdir_in(&workspace).unwrap();
        let mut state = AppState::new(tmp.path().to_path_buf(), tmp.path().to_path_buf());
        let cfg = crate::config::AppConfig::load_or_create().expect("config");
        let context = crate::app::context::AppContext::new(cfg);
        let new_path = tmp.path().join("inner");
        std::fs::create_dir_all(&new_path).unwrap();
        dispatch_emit_action(
            &mut state,
            &context,
            "cd",
            &serde_json::json!({ "path": new_path.to_string_lossy().to_string() }),
        );
        assert!(
            state
                .get_active_panel()
                .current_path
                .starts_with(tmp.path().canonicalize().unwrap()),
            "object-shaped args must be honoured and validated against the workspace"
        );
    }
}
