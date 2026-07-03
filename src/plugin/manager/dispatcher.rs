//! Dispatches `PluginRequest` values received from the plugin context
//! channel. Each variant mutates `AppState` (or, for the read-only
//! `GetStateSnapshot`, produces a snapshot value and sends it back via a
//! oneshot).
//!
//! The actual side-effect logic for the dispatchable variants lives in
//! `dispatch_actions.rs`; this file is the routing layer.

use super::dispatch_actions::dispatch_emit_action;
use super::request::{InputDialogResult, PluginRequest};
use super::snapshot::FileEntrySnapshot;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use std::path::PathBuf;

use super::dispatch_actions::compute_file_cache_path;

/// Processes plugin requests in the main application loop.
pub fn process_plugin_requests(state: &mut AppState, context: &AppContext) {
    if let Some(rx_mutex) = super::manager::PLUGIN_REQ_RX.get() {
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
                        state.active_popup = Some(PopupType::Info(format!("{}: {}", title, msg)));
                        log::info!("Plugin notify [{}]: {} - {}", level, title, msg);
                    }
                    PluginRequest::NotifyStructured(payload) => {
                        super::dispatch_actions::render_notify(state, &payload);
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
