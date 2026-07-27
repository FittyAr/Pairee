//! Dispatches `PluginRequest` values received from the plugin context
//! channel. Each variant mutates `AppState` (or, for the read-only
//! `GetStateSnapshot`, produces a snapshot value and sends it back via a
//! oneshot).
//!
//! The actual side-effect logic for the dispatchable variants lives in
//! `dispatch_actions.rs`; this file is the routing layer.

use super::dispatch_actions::dispatch_emit_action;
use super::request::PluginRequest;
use super::snapshot::FileEntrySnapshot;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use std::path::PathBuf;

use super::dispatch_actions::{compute_file_cache_path, validate_workspace_path};

/// Processes plugin requests in the main application loop.
///
/// Drains every pending message on
/// `state.plugin_req_rx` and dispatches each one to its
/// side-effect handler. The receiver is owned by `AppState`
/// directly, so this function does not need to lock a global
/// or compete with other readers — the only writer that
/// matters (the main loop) is the only one that calls it.
pub fn process_plugin_requests(state: &mut AppState, context: &AppContext) {
    while let Ok(req) = state.plugin_req_rx.try_recv() {
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

                let snapshot = super::snapshot::AppStateSnapshot {
                    active_panel: format!("{:?}", state.active_panel).to_lowercase(),
                    left_cwd: state.left_panel.current_path.to_string_lossy().to_string(),
                    right_cwd: state.right_panel.current_path.to_string_lossy().to_string(),
                    hovered_file: hovered,
                    selected_files: selected,
                };
                let _ = reply_tx.send(snapshot);
            }
            PluginRequest::Notify { title, msg, level } => {
                // Legacy `pairee.app.notify(title, msg, level)` path.
                //
                // We push the message into the non-modal `toast` slot rather
                // than the modal `PopupType::Info` slot, so the notification
                // no longer steals focus or blocks the active popup (e.g.
                // the plugin search panel stays open so the user can install
                // another plugin right away). A 3-second auto-dismiss is
                // applied by default — the structured `pairee.notify`
                // API lets callers pass an explicit `timeout_secs`.
                let body = format!("{}: {}", title, msg);
                super::dispatch_actions::push_toast(state, &body, &level, Some(3.0));
                log::info!("Plugin notify [{}]: {} - {}", level, title, msg);
            }
            PluginRequest::NotifyStructured(payload) => {
                super::dispatch_actions::render_notify(state, &payload);
            }
            PluginRequest::Cd { path } => {
                // §6 Secure-Mode equivalent: a plugin's `pairee.app.cd`
                // (and `pairee.emit("cd", ...)`) navigates the active
                // panel to a path of its choice. Without a workspace
                // check, a plugin could navigate the user to any
                // directory in the filesystem (e.g. `/etc`, the
                // user's `~/.ssh`, etc.) and then exfiltrate via
                // `pairee.fs.read` of the same path. We canonicalize
                // the path (which follows symlinks) and require it
                // to live inside the workspace / config / cache
                // directories — the same boundary that
                // `pairee.fs.read` enforces.
                let p = PathBuf::from(&path);
                match validate_workspace_path(&p) {
                    Some(canonical) => {
                        state.get_active_panel_mut().current_path = canonical;
                        state.refresh_both_panels(context.config.settings.show_hidden);
                    }
                    None => {
                        log::warn!(
                            "Plugin cd rejected: {:?} is outside the workspace / config / cache",
                            p
                        );
                    }
                }
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
                // M1: route the request to a real TUI popup. The
                // popup's key handler is responsible for sending the
                // result on `reply_tx` and clearing `active_popup`.
                // `position` is reserved for future use; today the
                // popup is always centered.
                log::debug!(
                    "Plugin input dialog requested: title={:?} position={:?} \
                         obscure={} realtime={} debounce={}s",
                    title,
                    position,
                    obscure,
                    realtime,
                    debounce_secs
                );
                let _ = position; // suppress unused warning
                state.active_popup = Some(PopupType::PluginInputDialog {
                    title,
                    input: default,
                    cursor_idx: 0,
                    obscure,
                    reply_tx: Some(reply_tx),
                });
            }
            PluginRequest::ConfirmDialog {
                title,
                msg,
                position,
                reply_tx,
            } => {
                // M1: real confirm popup. The key handler sends the
                // boolean reply when the user makes a choice.
                log::debug!(
                    "Plugin confirm dialog requested: title={:?}, msg={:?}, position={:?}",
                    title,
                    msg,
                    position
                );
                let _ = position;
                state.active_popup = Some(PopupType::PluginConfirmDialog {
                    title,
                    msg,
                    cursor_idx: 0,
                    reply_tx: Some(reply_tx),
                });
            }
            PluginRequest::WhichPrompt {
                candidates,
                silent,
                reply_tx,
            } => {
                // M1: real which-prompt. The key handler matches
                // candidate `on` keys (canonicalised via
                // `key_event_to_string`) and sends back the
                // 1-based index of the selected candidate.
                log::debug!(
                    "Plugin which-prompt requested for {} candidate(s), silent={}",
                    candidates.len(),
                    silent
                );
                state.active_popup = Some(PopupType::PluginWhichPrompt {
                    candidates,
                    silent,
                    reply_tx: Some(reply_tx),
                });
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
                use crate::fs::transfer::engine::TransferEngine;
                use crate::fs::transfer::job::{TransferJob, TransferOperation};
                use crate::fs::transfer::options::TransferOptions;

                let mut options = TransferOptions::default();
                options.verify_after_copy = context.config.settings.transfer_verify_after_copy;
                options.hash_algorithm =
                    match context.config.settings.transfer_default_hash.as_str() {
                        "crc32" => crate::fs::transfer::options::HashAlgorithm::Crc32,
                        "md5" => crate::fs::transfer::options::HashAlgorithm::Md5,
                        "sha1" => crate::fs::transfer::options::HashAlgorithm::Sha1,
                        "sha256" => crate::fs::transfer::options::HashAlgorithm::Sha256,
                        _ => crate::fs::transfer::options::HashAlgorithm::Blake3,
                    };
                options.buffer_size = match context.config.settings.transfer_buffer_size {
                    65536 => crate::fs::transfer::options::BufferSize::_64KB,
                    262144 => crate::fs::transfer::options::BufferSize::_256KB,
                    4194304 => crate::fs::transfer::options::BufferSize::_4MB,
                    _ => crate::fs::transfer::options::BufferSize::_1MB,
                };
                options.direct_io = context.config.settings.transfer_direct_io;
                options.preserve_timestamps = context.config.settings.transfer_preserve_timestamps;
                options.preserve_attributes = context.config.settings.transfer_preserve_attributes;
                options.preserve_acl = context.config.settings.transfer_preserve_acl;
                options.preserve_streams = context.config.settings.transfer_preserve_streams;
                options.skip_symlinks = context.config.settings.transfer_skip_symlinks;
                options.follow_symlinks = context.config.settings.transfer_follow_symlinks;
                options.limit_bandwidth_rate =
                    context.config.settings.transfer_limit_bandwidth_rate;
                options.halt_on_error = context.config.settings.transfer_halt_on_error;
                options.max_retries = context.config.settings.transfer_max_retries;
                options.conflict_resolution =
                    context.config.settings.transfer_conflict_resolution.clone();

                let job = TransferJob::new(TransferOperation::Copy, vec![from], to, options);

                if state.transfer.is_none() {
                    let (engine, rx) = TransferEngine::new();
                    state.transfer = Some(crate::app::state::transfer_state::TransferUIState::new(
                        engine, rx,
                    ));
                }

                if let Some(ref mut ts) = state.transfer {
                    ts.engine.submit_job(job);
                    ts.view_mode = crate::app::state::TransferViewMode::Minimized;
                }
            }
            PluginRequest::UpdatePluginWidget { path, widget } => {
                // Three cases:
                // 1. The QuickViewPanel is already open at
                //    this path — just swap the widget in.
                // 2. A different popup is open (or no popup) —
                //    activate the QuickViewPanel and seed it
                //    with the new widget. This makes
                //    `pairee.preview_widget` and `peek`
                //    return-renderable paths work without the
                //    user having to press F3 first.
                // 3. The widget doesn't apply — the path
                //    differs from the active panel; ignore.
                if let Some(PopupType::QuickViewPanel {
                    path: ref cur_path,
                    ref mut plugin_widget,
                    ..
                }) = state.active_popup
                {
                    if cur_path == &path {
                        *plugin_widget = Some(widget);
                    }
                } else {
                    // Activate a fresh preview pane with
                    // the widget. The empty `content` list
                    // signals "render the plugin widget,
                    // not the file body" to the renderer.
                    let widget = Some(widget);
                    state.active_popup = Some(PopupType::QuickViewPanel {
                        path,
                        content: Vec::new(),
                        scroll: 0,
                        image_data: None,
                        plugin_widget: widget,
                    });
                }
            }
            PluginRequest::PluginMenuLoaded {
                installed,
                registry,
            } => {
                if let Some(PopupType::PluginMenu {
                    installed: ref mut existing,
                    all_registry: ref mut existing_all,
                    registry: ref mut existing_registry,
                    installed_loading: ref mut loading,
                    installed_loading_status: ref mut loading_status,
                    ..
                }) = state.active_popup
                {
                    *existing = installed;
                    // all_registry stays as the full list for filtering
                    *existing_all = registry.clone();
                    // registry shows all entries until the user narrows it
                    *existing_registry = registry;
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
            PluginRequest::ImagePreview { path, rect } => {
                // M4 done-criterion: a plugin can call
                // `pairee.image.show(url, rect)` and see the
                // image render in the preview pane.
                // We decode the image here, stash it on
                // the QuickViewPanel's `image_data`, and
                // make the QuickViewPanel the active popup
                // (replacing whatever was there) so the
                // renderer picks it up.
                //
                // §6 TOCTOU: the binding validated the path
                // before sending the request, but a local
                // attacker could have swapped the symlink
                // between then and now. Re-validate here so
                // the I/O always targets the path the
                // binding agreed to decode.
                let validated = match super::dispatch_actions::validate_workspace_path(&path) {
                    Some(p) => p,
                    None => {
                        log::warn!(
                            "Plugin image preview rejected: {:?} is outside the workspace / \
                                 config / cache (TOCTOU re-check)",
                            path
                        );
                        return;
                    }
                };
                match image::open(&validated) {
                    Ok(img) => {
                        let qvp = PopupType::QuickViewPanel {
                            path: validated.clone(),
                            content: Vec::new(),
                            scroll: 0,
                            image_data: Some(img),
                            plugin_widget: None,
                        };
                        state.active_popup = Some(qvp);
                        log::info!(
                            "Plugin image preview rendered: path={:?} rect=({},{} {}x{})",
                            validated,
                            rect.x,
                            rect.y,
                            rect.w,
                            rect.h,
                        );
                    }
                    Err(e) => {
                        log::warn!(
                            "Plugin image preview failed to decode {:?}: {}",
                            validated,
                            e,
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// Build an `AppState` whose plugin request receiver is
    /// private to this test (no global `OnceLock` involved) and
    /// return the matching sender so the test can push requests
    /// directly. Each test gets its own channel pair, so
    /// parallel test execution can no longer leak messages
    /// across tests (the previous failure mode of the
    /// dispatcher tests).
    fn fresh_state_with_channel(
        cwd: &Path,
    ) -> (AppState, tokio::sync::mpsc::Sender<PluginRequest>) {
        let (tx, rx) = tokio::sync::mpsc::channel::<PluginRequest>(8);
        // `AppState::new` creates its own channel and writes
        // the sender into the `PLUGIN_REQ_TX` `OnceLock`;
        // we don't care about that one. We just need to
        // overwrite the receiver on the returned state
        // with our private one.
        let mut state = AppState::new(cwd.to_path_buf(), cwd.to_path_buf());
        state.plugin_req_rx = rx;
        (state, tx)
    }

    fn fresh_context(_cwd: &Path) -> AppContext {
        let cfg = crate::config::AppConfig::load_or_create().expect("config");
        AppContext::new(cfg)
    }

    #[tokio::test]
    async fn test_cd_accepts_workspace_path() {
        // The tempdir must be created inside the workspace
        // root (current_dir) so the workspace check accepts
        // the cd.
        let workspace = std::env::current_dir().unwrap();
        let tmp = tempfile::tempdir_in(&workspace).unwrap();
        let (mut state, tx) = fresh_state_with_channel(tmp.path());
        let context = fresh_context(tmp.path());
        let pre = state.get_active_panel().current_path.clone();
        let new_path = tmp.path().join("sub");
        std::fs::create_dir_all(&new_path).unwrap();
        tx.send(PluginRequest::Cd {
            path: new_path.to_string_lossy().to_string(),
        })
        .await
        .unwrap();
        process_plugin_requests(&mut state, &context);
        let post = state.get_active_panel().current_path.clone();
        assert_ne!(
            pre, post,
            "active panel cwd should have changed after workspace cd"
        );
        assert!(
            post.starts_with(tmp.path().canonicalize().unwrap()),
            "post-cd cwd must remain inside the workspace"
        );
    }

    #[tokio::test]
    async fn test_cd_rejects_outside_workspace() {
        // Sending a Cd request to a path outside the
        // workspace / config / cache roots must NOT mutate
        // the active panel. The dispatcher logs a warn and
        // returns early.
        let tmp = tempfile::tempdir().unwrap();
        let (mut state, tx) = fresh_state_with_channel(tmp.path());
        let context = fresh_context(tmp.path());
        let pre = state.get_active_panel().current_path.clone();
        tx.send(PluginRequest::Cd {
            path: PathBuf::from("/etc/should-be-rejected")
                .to_string_lossy()
                .to_string(),
        })
        .await
        .unwrap();
        process_plugin_requests(&mut state, &context);
        let post = state.get_active_panel().current_path.clone();
        assert_eq!(
            pre, post,
            "active panel cwd must NOT change after outside-workspace cd"
        );
    }
}
