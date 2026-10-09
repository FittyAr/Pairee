//! Dispatches `PluginRequest` values received from the plugin context
//! channel. Each variant mutates `AppState` (or, for the read-only
//! `GetStateSnapshot`, produces a snapshot value and sends it back via a
//! oneshot).
//!
//! The actual side-effect logic for the dispatchable variants lives in
//! `dispatch_actions.rs`; this file is the routing layer: one small handler
//! per request group.

use super::dispatch_actions::dispatch_emit_action;
use super::request::PluginRequest;
use super::snapshot::{AppStateSnapshot, FileEntrySnapshot};
use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PendingPluginReply, PopupType};
use std::path::PathBuf;

use super::dispatch_actions::compute_file_cache_path;

/// Processes plugin requests in the main application loop.
pub fn process_plugin_requests(state: &mut AppState, context: &AppContext) {
    super::dialogs::settle_orphaned_plugin_dialogs(state);

    if let Some(rx_mutex) = super::lifecycle::PLUGIN_REQ_RX.get()
        && let Ok(mut rx) = rx_mutex.try_lock()
    {
        while let Ok(req) = rx.try_recv() {
            dispatch_request(state, context, req);
        }
    }
}

/// Routes one request to its handler.
fn dispatch_request(state: &mut AppState, context: &AppContext, req: PluginRequest) {
    match req {
        PluginRequest::GetStateSnapshot(reply_tx) => {
            let _ = reply_tx.send(build_snapshot(state));
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
            state.get_active_panel_mut().current_path = PathBuf::from(path);
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
        PluginRequest::SetFocus { side } => set_focus(state, &side),
        PluginRequest::EmitAction {
            name,
            args,
            reply_tx,
        } => {
            dispatch_emit_action(state, context, &name, &args);
            // Emit is fire-and-forget for the caller; send `null` so the
            // awaiting binding returns at once.
            if let Some(tx) = reply_tx {
                let _ = tx.send(serde_json::Value::Null);
            }
        }
        PluginRequest::FileCache {
            file_path,
            skip,
            reply_tx,
        } => {
            let _ = reply_tx.send(compute_file_cache_path(&file_path, skip));
        }
        PluginRequest::SpawnCopyTask { from, to } => spawn_copy_task(state, context, from, to),
        other => dispatch_dialog_request(state, other),
    }
}

/// Requests that open plugin dialogs.
fn dispatch_dialog_request(state: &mut AppState, req: PluginRequest) {
    match req {
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
            // Realtime streaming needs an mpsc Recv (later). The dialog still
            // honours obscure + default + submit/cancel.
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
        } => super::dialogs::open_confirm(state, title, msg, position, reply_tx),
        PluginRequest::WhichPrompt {
            candidates,
            silent,
            reply_tx,
        } => super::dialogs::open_which(state, candidates, silent, reply_tx),
        other => dispatch_popup_update(state, other),
    }
}

/// Requests that refresh data shown by an open popup.
fn dispatch_popup_update(state: &mut AppState, req: PluginRequest) {
    match req {
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
            if let Some(PopupType::PluginMenu(menu)) = state.dialogs.top_mut() {
                menu.installed = installed;
                menu.all_registry = registry.clone();
                menu.registry = registry;
                menu.installed_loading = false;
                menu.installed_loading_status = String::new();
            }
        }
        PluginRequest::DevPluginScan { options } => open_dev_plugin_select(state, options),
        // The remaining variants are routed by the two dispatchers above.
        _ => {}
    }
}

/// Read-only view of the panels for `pairee.app.state()`.
fn build_snapshot(state: &AppState) -> AppStateSnapshot {
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
    let cwd = |side| {
        state
            .panels
            .side(side)
            .current_path
            .to_string_lossy()
            .to_string()
    };
    AppStateSnapshot {
        active_panel: format!("{:?}", state.panels.active).to_lowercase(),
        left_cwd: cwd(ActivePanel::Left),
        right_cwd: cwd(ActivePanel::Right),
        hovered_file: hovered,
        selected_files: selected,
    }
}

fn set_focus(state: &mut AppState, side: &str) {
    match side {
        "left" => state.panels.active = ActivePanel::Left,
        "right" => state.panels.active = ActivePanel::Right,
        _ => {}
    }
}

/// Queues a copy job built from the transfer settings and minimizes the transfer bar.
fn spawn_copy_task(state: &mut AppState, context: &AppContext, from: PathBuf, to: PathBuf) {
    use crate::fs::transfer::job::{TransferJob, TransferOperation};
    use crate::fs::transfer::options_from_settings::transfer_options_from_settings;

    log::info!("Plugin requesting copy from {:?} to {:?}", from, to);
    let options = transfer_options_from_settings(&context.config.settings);
    let job = TransferJob::new(TransferOperation::Copy, vec![from], to, options);

    crate::fs::transfer::submit::ensure_transfer_ui(state);
    if let Some(ref mut ts) = state.transfer {
        ts.engine.submit_job(job);
        ts.view_mode = crate::app::state::TransferViewMode::Minimized;
    }
}

/// Converts a dev-plugin scan into an open `SelectDevPlugin` popup.
fn open_dev_plugin_select(state: &mut AppState, options: Vec<(String, String)>) {
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
