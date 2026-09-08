//! Background channel event processors for terminal, SSH, search, and plugin dev tools.

use crate::app::context::AppContext;
use crate::app::state::{AppState, DevProgress, PopupType, Screen};
use crate::terminal::TerminalBackend;

pub fn process_terminal_updates(state: &mut AppState) {
    if let Some(rx) = state.term_rx.as_mut() {
        let mut got = false;
        while let Ok(update) = rx.try_recv() {
            got = true;
            if let Some(Screen::Terminal(ts)) = state.screens.get_mut(update.screen_idx) {
                match update.line {
                    Some(line) => ts.output_lines.push(line),
                    None => ts.is_running = false,
                }
            }
        }
        if got {
            state.mark_ui_dirty();
        }
    }
}

pub fn process_ssh_connect_updates(state: &mut AppState, context: &AppContext) {
    let ssh_done = state
        .ssh_connect_rx
        .as_mut()
        .map(|rx| rx.try_recv())
        .unwrap_or(Err(tokio::sync::oneshot::error::TryRecvError::Empty));
    match ssh_done {
        Ok((panel, res)) => {
            state.ssh_connect_rx = None;
            match res {
                Ok(client) => {
                    let p = match panel {
                        crate::app::state::ActivePanel::Left => &mut state.panels.left,
                        crate::app::state::ActivePanel::Right => &mut state.panels.right,
                    };
                    p.ssh_conn = Some(client);
                    p.current_path = std::path::PathBuf::from("/");
                    p.cursor_index = 0;
                    p.clear_selection();
                    state.dialogs.clear();
                    state.refresh_both_panels(context.config.settings.show_hidden);
                    state.mark_ui_dirty();
                }
                Err(e) => {
                    state.dialogs.replace(PopupType::Error(format!(
                        "{} {}",
                        crate::config::localization::t("error_ssh_failed"),
                        e
                    )));
                    state.mark_ui_dirty();
                }
            }
        }
        Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
        Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
            state.ssh_connect_rx = None;
        }
    }
}

pub fn process_search_updates(state: &mut AppState) {
    if let Some(rx) = state.search_rx.as_mut() {
        let mut new_results = Vec::new();
        let mut closed = false;
        loop {
            match rx.try_recv() {
                Ok((path, is_dir)) => {
                    new_results.push((path, is_dir));
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    break;
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    closed = true;
                    break;
                }
            }
        }
        if !new_results.is_empty()
            && let Some(PopupType::SearchResults { results, .. }) = state.dialogs.top_mut()
        {
            for (path, is_dir) in new_results {
                if results.len() < 500 {
                    results.push((path, is_dir));
                } else {
                    closed = true;
                    break;
                }
            }
        }
        if closed {
            if let Some(PopupType::SearchResults { searching, .. }) = state.dialogs.top_mut() {
                *searching = false;
            }
            state.search_rx = None;
        }
        state.mark_ui_dirty();
    }
}

pub fn process_dev_progress_updates(state: &mut AppState) {
    if let Some(rx) = state.plugins.dev_progress_rx.as_mut() {
        let mut latest: Option<DevProgress> = None;
        let mut finished: Option<DevProgress> = None;
        let mut disconnected = false;
        loop {
            match rx.try_recv() {
                Ok(update) => {
                    if update.done {
                        finished = Some(update);
                        latest = None;
                    } else {
                        latest = Some(update);
                    }
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
        if let Some(update) = finished {
            if let Some(PopupType::PluginMenu(menu)) = state.dialogs.top_mut() {
                if let Some(err) = update.error {
                    menu.dev_results = err;
                } else if let Some(res) = update.result {
                    menu.dev_results = res;
                }
                menu.dev_loading = false;
                menu.dev_loading_status = String::new();
                menu.dev_loading_progress = None;
            }
        } else if let Some(update) = latest
            && let Some(PopupType::PluginMenu(menu)) = state.dialogs.top_mut()
        {
            menu.dev_loading = true;
            menu.dev_loading_status = update.status;
            menu.dev_loading_progress = if let (Some(c), Some(t)) = (update.current, update.total) {
                if t > 0 { Some((c, t)) } else { None }
            } else {
                None
            };
        }
        if disconnected {
            state.plugins.dev_progress_rx = None;
        }
    }
}

pub fn process_pending_custom_command(
    state: &mut AppState,
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) {
    if let Some(cmd) = state.pending_custom_command.take() {
        let active_path = state.get_active_panel().current_path.clone();
        let _ = crate::app::actions::exec::execute_shell_command(
            &cmd,
            &active_path,
            context,
            terminal_backend,
        );
        state.refresh_both_panels(context.config.settings.show_hidden);
    }
}
