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
use crate::app::state::{AppState, PendingPluginReply, PopupType};
use std::path::PathBuf;

use super::dispatch_actions::compute_file_cache_path;

/// Processes plugin requests in the main application loop.
pub fn process_plugin_requests(state: &mut AppState, context: &AppContext) {
    super::dialogs::settle_orphaned_plugin_dialogs(state);

    if let Some(rx_mutex) = super::lifecycle::PLUGIN_REQ_RX.get()
        && let Ok(mut rx) = rx_mutex.try_lock()
    {
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
                        active_panel: format!("{:?}", state.panels.active).to_lowercase(),
                        left_cwd: state.panels.left.current_path.to_string_lossy().to_string(),
                        right_cwd: state
                            .panels
                            .right
                            .current_path
                            .to_string_lossy()
                            .to_string(),
                        hovered_file: hovered,
                        selected_files: selected,
                    };
                    let _ = reply_tx.send(snapshot);
                }
                PluginRequest::Notify { title, msg, level } => {
                    state
                        .dialogs
                        .replace(PopupType::Info(format!("{}: {}", title, msg)));
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
                        state.panels.active = crate::app::state::ActivePanel::Left;
                    } else if side == "right" {
                        state.panels.active = crate::app::state::ActivePanel::Right;
                    }
                }
                PluginRequest::Confirm {
                    title,
                    msg,
                    reply_tx,
                } => {
                    log::warn!(
                        "Plugin called deprecated `pairee.app.confirm(title, msg)`; \
                             migrate to `pairee.confirm({{ pos = ..., title = ..., body = ... }}) \
                             for a real dialog."
                    );
                    super::dialogs::open_confirm(state, title, msg, None, reply_tx);
                }
                PluginRequest::Input {
                    title,
                    default,
                    reply_tx,
                } => {
                    log::warn!(
                        "Plugin called deprecated `pairee.app.input(title, default)`; \
                             migrate to `pairee.input({{ pos = ..., title = ..., value = ..., \
                             obscure = ..., realtime = ..., debounce = ... }}) for a real dialog."
                    );
                    super::dialogs::open_input(
                        state,
                        title,
                        default,
                        false,
                        None,
                        PendingPluginReply::LegacyInput(reply_tx),
                    );
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
                    log::info!(
                        "Plugin input dialog: title={:?} obscure={} realtime={} debounce={}s",
                        title,
                        obscure,
                        realtime,
                        debounce_secs
                    );
                    // Realtime streaming needs an mpsc Recv (later). The
                    // dialog still honours obscure + default + submit/cancel.
                    let _ = realtime;
                    super::dialogs::open_input(
                        state,
                        title,
                        default,
                        obscure,
                        position,
                        PendingPluginReply::Input(reply_tx),
                    );
                }
                PluginRequest::ConfirmDialog {
                    title,
                    msg,
                    position,
                    reply_tx,
                } => {
                    super::dialogs::open_confirm(state, title, msg, position, reply_tx);
                }
                PluginRequest::WhichPrompt {
                    candidates,
                    silent,
                    reply_tx,
                } => {
                    super::dialogs::open_which(state, candidates, silent, reply_tx);
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
                        1048576 => crate::fs::transfer::options::BufferSize::_1MB,
                        4194304 => crate::fs::transfer::options::BufferSize::_4MB,
                        n if n <= 65536 => crate::fs::transfer::options::BufferSize::_64KB,
                        n if n <= 262144 => crate::fs::transfer::options::BufferSize::_256KB,
                        n if n <= 1048576 => crate::fs::transfer::options::BufferSize::_1MB,
                        _ => crate::fs::transfer::options::BufferSize::_4MB,
                    };
                    options.direct_io = context.config.settings.transfer_direct_io;
                    options.preserve_timestamps =
                        context.config.settings.transfer_preserve_timestamps;
                    options.preserve_attributes =
                        context.config.settings.transfer_preserve_attributes;
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
                        state.transfer = Some(
                            crate::app::state::transfer_state::TransferUIState::new(engine, rx),
                        );
                    }

                    if let Some(ref mut ts) = state.transfer {
                        ts.engine.submit_job(job);
                        ts.view_mode = crate::app::state::TransferViewMode::Minimized;
                    }
                }
                PluginRequest::UpdatePluginWidget { path, widget } => {
                    if let Some(PopupType::QuickViewPanel(qv)) = state.dialogs.top_mut()
                        && qv.path == path
                    {
                        qv.plugin_widget = Some(widget);
                    }
                }
                PluginRequest::PluginMenuLoaded {
                    installed,
                    registry,
                } => {
                    if let Some(PopupType::PluginMenu {
                        installed: existing,
                        all_registry: existing_all,
                        registry: existing_registry,
                        installed_loading: loading,
                        installed_loading_status: loading_status,
                        ..
                    }) = state.dialogs.top_mut()
                    {
                        *existing = installed;
                        *existing_all = registry.clone();
                        *existing_registry = registry;
                        *loading = false;
                        *loading_status = String::new();
                    }
                }
                PluginRequest::DevPluginScan { options } => {
                    // Convert the scan into an open SelectDevPlugin popup.
                    let previous_popup = state
                        .dialogs
                        .top()
                        .cloned()
                        .map(Box::new)
                        .unwrap_or_else(|| Box::new(PopupType::Info(String::new())));
                    state.dialogs.replace(PopupType::SelectDevPlugin {
                        options,
                        cursor_idx: 0,
                        previous_popup,
                    });
                }
            }
        }
    }
}
