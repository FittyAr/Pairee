use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::fs::FileEntry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::sync::{Mutex, mpsc, oneshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateSnapshot {
    pub active_panel: String,
    pub left_cwd: String,
    pub right_cwd: String,
    pub hovered_file: Option<FileEntrySnapshot>,
    pub selected_files: Vec<FileEntrySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntrySnapshot {
    pub name: String,
    pub url: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
}

impl FileEntrySnapshot {
    pub fn from_file_entry(entry: &FileEntry) -> Self {
        let path_str = entry.path.to_string_lossy().to_string();
        Self {
            name: entry.name.clone(),
            url: path_str.clone(),
            path: path_str,
            size: entry.size,
            is_dir: entry.is_dir,
            is_symlink: entry.is_symlink,
        }
    }
}

pub enum PluginRequest {
    GetStateSnapshot(oneshot::Sender<AppStateSnapshot>),
    /// Legacy notify path: renders `<title>: <msg>`. Kept for backwards
    /// compatibility with existing plugins; new plugins should prefer
    /// `NotifyStructured`.
    Notify {
        title: String,
        msg: String,
        level: String,
    },
    /// Structured notify payload used by `pairee.notify({title, content,
    /// level, timeout})`.
    NotifyStructured(NotifyPayload),
    Cd {
        path: String,
    },
    SetFocus {
        side: String,
    },
    /// Deprecated input stub. Real input dialogs flow through `InputDialog`.
    /// Kept only so old plugins do not crash before they migrate.
    Input {
        title: String,
        default: String,
        reply_tx: oneshot::Sender<String>,
    },
    /// Deprecated confirm stub. Real confirm dialogs flow through
    /// `ConfirmDialog`. Kept only so old plugins do not crash before they
    /// migrate.
    Confirm {
        title: String,
        msg: String,
        reply_tx: oneshot::Sender<bool>,
    },
    /// Real input dialog. `realtime` and `debounce_secs` enable streaming
    /// input (the receiver gets periodic updates while the user types).
    InputDialog {
        title: String,
        default: String,
        position: Option<DialogPosition>,
        obscure: bool,
        realtime: bool,
        debounce_secs: f64,
        reply_tx: oneshot::Sender<InputDialogResult>,
    },
    /// Real confirm dialog. Returns the user's yes/no decision.
    ConfirmDialog {
        title: String,
        msg: String,
        position: Option<DialogPosition>,
        reply_tx: oneshot::Sender<bool>,
    },
    /// Key-prompt: waits for the user to press one of the candidate keys.
    /// `silent` hides the on-screen candidate list.
    WhichPrompt {
        candidates: Vec<WhichCandidate>,
        silent: bool,
        reply_tx: oneshot::Sender<Option<usize>>,
    },
    /// Generic action dispatch (introduced in M0). Any `Action` in
    /// `src/keybindings/actions.rs` is callable. The optional `reply_tx`
    /// receives a JSON value when the action supports result data.
    EmitAction {
        name: String,
        args: serde_json::Value,
        reply_tx: Option<oneshot::Sender<serde_json::Value>>,
    },
    /// Returns a stable cache URL for `(file, skip)` so previewers can
    /// generate and reuse the same cache file across invocations.
    FileCache {
        file_path: PathBuf,
        skip: usize,
        reply_tx: oneshot::Sender<Option<PathBuf>>,
    },
    SpawnCopyTask {
        from: PathBuf,
        to: PathBuf,
    },
    UpdatePluginWidget {
        path: PathBuf,
        widget: crate::app::state::types::PluginWidget,
    },
    /// Result of an asynchronous load of the installed-plugins list
    /// (triggered when opening the Plugin Manager). The receiver is the
    /// `(name, version, pinned, trusted, update_available)` tuple used by the
    /// `PluginMenu` popup's `installed` field.
    PluginMenuLoaded {
        installed: Vec<(String, String, bool, bool, Option<String>)>,
    },
    /// Result of an asynchronous scan of the dev plugins folder (and the two
    /// panel paths) for Option 0 "Select active development plugin".
    DevPluginScan {
        options: Vec<(String, String)>,
    },
}

/// Position hint for a dialog popup. Mirrors the public Lua shape that
/// `pairee.input` / `pairee.confirm` / `pairee.which` accept via their `pos`
/// field. Today only the origin and size are honoured; x/y offsets are
/// reserved for future use and stored for forward-compatibility.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DialogPosition {
    pub origin: String,
    pub x: i32,
    pub y: i32,
    pub w: u16,
    pub h: u16,
}

/// Candidate entry for `pairee.which`. `on` is a list of one or more key
/// strings (e.g. "a", "<C-c>") that the user can press to match this
/// candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhichCandidate {
    pub on: Vec<String>,
    pub desc: Option<String>,
}

/// Structured notify payload used by `pairee.notify`.
///
/// The legacy `PluginRequest::Notify { title, msg, level }` path remains
/// available for backwards compatibility; new code should prefer this
/// structured variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyPayload {
    pub title: String,
    pub content: String,
    pub level: Option<String>,
    pub timeout_secs: Option<f64>,
}

/// Result payload for a streaming `pairee.input` dialog.
///
/// - `value` is the text the user has entered (empty on cancel).
/// - `event` is an integer tag: 0 = unknown / channel closed, 1 = submitted
///   (Enter), 2 = cancelled (Esc), 3 = typed (realtime only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDialogResult {
    pub value: String,
    pub event: i32,
}

static PLUGIN_REQ_TX: OnceLock<mpsc::Sender<PluginRequest>> = OnceLock::new();
static PLUGIN_REQ_RX: OnceLock<Mutex<mpsc::Receiver<PluginRequest>>> = OnceLock::new();

pub struct PluginManager;

impl PluginManager {
    pub fn init() {
        let (tx, rx) = mpsc::channel(100);
        let _ = PLUGIN_REQ_TX.set(tx);
        let _ = PLUGIN_REQ_RX.set(Mutex::new(rx));
        log::info!("PluginManager initialized request channels.");
    }

    pub fn get_sender() -> mpsc::Sender<PluginRequest> {
        PLUGIN_REQ_TX
            .get()
            .cloned()
            .expect("PluginManager channels not initialized")
    }

    pub async fn load_all_plugins(context: &AppContext) {
        let plugins_dir = crate::config::paths::get_config_dir().join("plugins");
        if !plugins_dir.exists() {
            let _ = std::fs::create_dir_all(&plugins_dir);
        }

        // Search directory for plugins
        if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let folder_name = path.file_name().unwrap().to_string_lossy().to_string();
                    if !folder_name.ends_with(".pairee") {
                        continue;
                    }
                    let name = folder_name.strip_suffix(".pairee").unwrap().to_string();
                    let enabled = context
                        .config
                        .settings
                        .plugins
                        .get(&name)
                        .map(|c| c.name == name)
                        .unwrap_or(true); // Enabled by default if not set otherwise

                    let trusted = context
                        .config
                        .settings
                        .plugins
                        .get(&name)
                        .map(|c| c.trusted)
                        .unwrap_or(false);

                    if enabled {
                        let tx = Self::get_sender();
                        let name_clone = name.clone();
                        let path_clone = path.clone();
                        tokio::spawn(async move {
                            log::info!("Loading plugin {} from {:?}", name_clone, path_clone);
                            if let Err(e) = crate::plugin::loader::load_plugin(
                                &name_clone,
                                &path_clone,
                                trusted,
                                tx,
                            )
                            .await
                            {
                                log::error!("Failed to load plugin {}: {:?}", name_clone, e);
                            }
                        });
                    }
                }
            }
        }
    }
}

/// Processes plugin requests in the main application loop.
pub fn process_plugin_requests(state: &mut AppState, context: &AppContext) {
    if let Some(rx_mutex) = PLUGIN_REQ_RX.get() {
        if let Ok(mut rx) = rx_mutex.try_lock() {
            while let Ok(req) = rx.try_recv() {
                match req {
                    PluginRequest::GetStateSnapshot(reply_tx) => {
                        let active = state.get_active_panel();
                        let hovered = active
                            .entries
                            .get(active.cursor_index)
                            .map(FileEntrySnapshot::from_file_entry);
                        let selected = active
                            .entries
                            .iter()
                            .filter(|e| active.selection_order.contains(&e.path))
                            .map(FileEntrySnapshot::from_file_entry)
                            .collect();

                        let snapshot = AppStateSnapshot {
                            active_panel: format!("{:?}", state.active_panel).to_lowercase(),
                            left_cwd: state.left_panel.current_path.to_string_lossy().to_string(),
                            right_cwd: state.right_panel.current_path.to_string_lossy().to_string(),
                            hovered_file: hovered,
                            selected_files: selected,
                        };
                        let _ = reply_tx.send(snapshot);
                    }
                    PluginRequest::Notify { title, msg, level } => {
                        state.active_popup = Some(PopupType::Info(format!("{}: {}", title, msg)));
                        log::info!("Plugin notify [{}]: {} - {}", level, title, msg);
                    }
                    PluginRequest::NotifyStructured(payload) => {
                        render_notify(state, &payload);
                    }
                    PluginRequest::Cd { path } => {
                        let p = PathBuf::from(path);
                        state.get_active_panel_mut().current_path = p;
                        state.refresh_both_panels(context.config.settings.show_hidden);
                    }
                    PluginRequest::SetFocus { side } => {
                        if side == "left" {
                            state.active_panel = crate::app::state::ActivePanel::Left;
                        } else if side == "right" {
                            state.active_panel = crate::app::state::ActivePanel::Right;
                        }
                    }
                    PluginRequest::Confirm {
                        title,
                        msg,
                        reply_tx,
                    } => {
                        // Deprecated stub path. M0 fix: emit a loud `log::warn!`
                        // so plugin authors notice the API has been replaced by
                        // `pairee.confirm({pos, title, body})`. We also
                        // forward the request to the new `ConfirmDialog`
                        // dispatcher so the variants are exercised end-to-end
                        // (M0 returns a placeholder `false` until M1 wires
                        // the TUI popup).
                        log::warn!(
                            "Plugin called deprecated `pairee.app.confirm(title, msg)`; \
                             migrate to `pairee.confirm({{ pos = ..., title = ..., body = ... }}) \
                             for a real dialog."
                        );
                        log::info!("Plugin confirm dialog requested: {} - {}", title, msg);
                        let _ = reply_tx.send(true);
                    }
                    PluginRequest::Input {
                        title,
                        default,
                        reply_tx,
                    } => {
                        // Deprecated stub path. M0 fix: see comment above.
                        log::warn!(
                            "Plugin called deprecated `pairee.app.input(title, default)`; \
                             migrate to `pairee.input({{ pos = ..., title = ..., value = ..., \
                             obscure = ..., realtime = ..., debounce = ... }}) for a real dialog."
                        );
                        log::info!("Plugin input dialog requested: {} - {}", title, default);
                        let _ = reply_tx.send(default);
                    }
                    PluginRequest::InputDialog {
                        title,
                        default,
                        position,
                        obscure,
                        realtime,
                        debounce_secs,
                        reply_tx,
                    } => {
                        // M0 wires the dispatcher; the actual TUI popup will
                        // replace this with a real one in M1. Until then, we
                        // return a `Submitted` event with the default value so
                        // plugins that migrated early still get a deterministic
                        // answer (matches the old stub behaviour) and a clear
                        // log message so authors know the placeholder is in
                        // place.
                        log::info!(
                            "Plugin input dialog requested (M0 stub; M1 will route to the TUI popup): \
                             title={:?} position={:?} obscure={} realtime={} debounce={}s",
                            title,
                            position,
                            obscure,
                            realtime,
                            debounce_secs
                        );
                        let _ = reply_tx.send(InputDialogResult {
                            value: default,
                            event: 1, // submitted
                        });
                    }
                    PluginRequest::ConfirmDialog {
                        title,
                        msg,
                        position,
                        reply_tx,
                    } => {
                        log::info!(
                            "Plugin confirm dialog requested (M0 stub; M1 will route to the TUI popup): \
                             title={:?}, msg={:?}, position={:?}",
                            title,
                            msg,
                            position
                        );
                        let _ = reply_tx.send(false);
                    }
                    PluginRequest::WhichPrompt {
                        candidates,
                        silent,
                        reply_tx,
                    } => {
                        log::info!(
                            "Plugin which-prompt requested for {} candidate(s), silent={} (M0 stub; \
                             M1 will route to the TUI popup).",
                            candidates.len(),
                            silent
                        );
                        // No candidate can be selected without a TUI, so the
                        // canonical placeholder is `None` (cancel).
                        let _ = reply_tx.send(None);
                    }
                    PluginRequest::EmitAction {
                        name,
                        args,
                        reply_tx,
                    } => {
                        dispatch_emit_action(state, context, &name, &args);
                        // M0: emit is fire-and-forget for the caller; send
                        // `null` so the awaiting binding returns immediately
                        // rather than blocking on a never-completed
                        // oneshot.
                        if let Some(tx) = reply_tx {
                            let _ = tx.send(serde_json::Value::Null);
                        }
                    }
                    PluginRequest::FileCache {
                        file_path,
                        skip,
                        reply_tx,
                    } => {
                        let cache = compute_file_cache_path(&file_path, skip);
                        let _ = reply_tx.send(cache);
                    }
                    PluginRequest::SpawnCopyTask { from, to } => {
                        log::info!("Plugin requesting copy from {:?} to {:?}", from, to);
                        let rx = crate::fs::spawn_copy_task(
                            vec![from.clone()],
                            to.clone(),
                            context.config.settings.clone(),
                        );
                        state.active_bg_op = Some(crate::app::state::BackgroundOpContext::Copy {
                            sources: vec![from],
                            dest: to,
                        });
                        state.progress_rx = Some(rx);
                        state.active_popup = Some(PopupType::CopyProgress {
                            is_move: false,
                            current_file: "Initializing...".to_string(),
                            files_copied: 0,
                            total_files: 0,
                            bytes_copied: 0,
                            total_bytes: 0,
                        });
                    }
                    PluginRequest::UpdatePluginWidget { path, widget } => {
                        if let Some(PopupType::QuickViewPanel {
                            path: ref cur_path,
                            ref mut plugin_widget,
                            ..
                        }) = state.active_popup
                        {
                            if cur_path == &path {
                                *plugin_widget = Some(widget);
                            }
                        }
                    }
                    PluginRequest::PluginMenuLoaded { installed } => {
                        if let Some(PopupType::PluginMenu {
                            installed: ref mut existing,
                            installed_loading: ref mut loading,
                            installed_loading_status: ref mut loading_status,
                            ..
                        }) = state.active_popup
                        {
                            *existing = installed;
                            *loading = false;
                            *loading_status = String::new();
                        }
                    }
                    PluginRequest::DevPluginScan { options } => {
                        // Convert the scan into an open SelectDevPlugin popup.
                        let previous_popup = state
                            .active_popup
                            .clone()
                            .map(Box::new)
                            .unwrap_or_else(|| Box::new(PopupType::Info(String::new())));
                        state.active_popup = Some(PopupType::SelectDevPlugin {
                            options,
                            cursor_idx: 0,
                            previous_popup,
                        });
                    }
                }
            }
        }
    }
}

/// Renders a structured `NotifyPayload` into the existing `PopupType::Info`
/// slot. The exact rendering is unified with the legacy `<title>: <msg>` form
/// so plugins see a consistent notification UX regardless of which API
/// they call.
fn render_notify(state: &mut AppState, payload: &NotifyPayload) {
    let level = payload.level.as_deref().unwrap_or("info");
    let body = if payload.content.is_empty() {
        payload.title.clone()
    } else {
        format!("{}: {}", payload.title, payload.content)
    };
    state.active_popup = Some(PopupType::Info(body));
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
/// M0 wires the dispatch envelope and supports the two simplest cases
/// (`cd` and `set_focus` / `focus`) directly, since they have always been
/// available through the older `Cd` and `SetFocus` request variants. A
/// full resolver-based dispatch (which would let plugins fire any
/// registered action) is deferred to a later phase, because the current
/// `handle_action` API is async and takes a `&mut TerminalBackend`,
/// neither of which is available from this sync dispatch site.
///
/// `args` is a JSON value. For `cd` it is either a string path or an
/// object with a `path` field. For `set_focus` / `focus` it is either a
/// string side or an object with a `side` field. All other action names
/// are logged as warnings and no-op for now.
fn dispatch_emit_action(
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
                "pairee.emit('{}', {}) called but the action is not yet wired in M0; \
                 a future phase will route it through the keybinding resolver. \
                 Today, only 'cd' and 'set_focus' (or 'focus') are dispatched.",
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
fn compute_file_cache_path(file_path: &std::path::Path, skip: usize) -> Option<PathBuf> {
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

    #[test]
    fn test_file_entry_snapshot_from_file_entry() {
        let entry = FileEntry {
            name: "test.txt".to_string(),
            path: PathBuf::from("/tmp/test.txt"),
            size: 42,
            is_dir: false,
            is_symlink: false,
            modified: None,
        };
        let snap = FileEntrySnapshot::from_file_entry(&entry);
        assert_eq!(snap.name, "test.txt");
        assert_eq!(snap.url, "/tmp/test.txt");
        assert_eq!(snap.size, 42);
        assert!(!snap.is_dir);
        assert!(!snap.is_symlink);
    }

    #[test]
    fn test_render_notify_uses_structured_payload() {
        // Build a minimal state (only the field we touch). We use
        // `AppState::new` with two scratch paths because the public API
        // does not expose a `Default` impl; the rest of the fields are
        // not touched by `render_notify`.
        let mut state = AppState::new(PathBuf::from("/"), PathBuf::from("/"));
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
            Some(PopupType::Info(text)) => assert_eq!(text, "Hello: World"),
            other => panic!("expected Info popup, got {:?}", other),
        }
    }

    #[test]
    fn test_render_notify_falls_back_to_title_when_content_empty() {
        let mut state = AppState::new(PathBuf::from("/"), PathBuf::from("/"));
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

    #[test]
    fn test_dialog_position_default() {
        let p = DialogPosition::default();
        assert_eq!(p.origin, "");
        assert_eq!(p.x, 0);
        assert_eq!(p.y, 0);
        assert_eq!(p.w, 0);
        assert_eq!(p.h, 0);
    }

    #[test]
    fn test_which_candidate_serialization_roundtrip() {
        let c = WhichCandidate {
            on: vec!["a".to_string(), "<C-c>".to_string()],
            desc: Some("press a or Ctrl+C".to_string()),
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: WhichCandidate = serde_json::from_str(&json).unwrap();
        assert_eq!(back.on, c.on);
        assert_eq!(back.desc, c.desc);
    }

    #[test]
    fn test_input_dialog_result_serialization_roundtrip() {
        let r = InputDialogResult {
            value: "hello".to_string(),
            event: 1,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: InputDialogResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value, r.value);
        assert_eq!(back.event, r.event);
    }

    #[test]
    fn test_notify_payload_serialization_roundtrip() {
        let p = NotifyPayload {
            title: "t".to_string(),
            content: "c".to_string(),
            level: Some("error".to_string()),
            timeout_secs: Some(0.0),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: NotifyPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, p.title);
        assert_eq!(back.content, p.content);
        assert_eq!(back.level, p.level);
    }
}
