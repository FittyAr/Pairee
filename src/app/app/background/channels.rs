//! Background channel event processors for terminal, SSH, search, and plugin dev tools.

use crate::app::context::AppContext;
use crate::app::state::popup::SearchResults;
use crate::app::state::{AppState, DevProgress, PopupType, Screen};
use crate::terminal::TerminalBackend;

/// New output and exits of the terminal screens' processes.
pub fn process_terminal_updates(state: &mut AppState) {
    let mut changed = false;
    for screen in state.screens.iter_mut() {
        if let Screen::Terminal(ts) = screen {
            changed |= ts.poll();
        }
    }
    if changed {
        state.mark_ui_dirty();
    }
}

pub fn process_ssh_connect_updates(state: &mut AppState, context: &AppContext) {
    let Some((tab, res)) = state.ssh_connect.poll() else {
        return;
    };
    let show_hidden = context.config.settings.show_hidden;
    if crate::app::session::remote::finish(state, tab, &res, show_hidden) {
        state.mark_ui_dirty();
        return;
    }
    match res {
        // The tab was closed while connecting: drop the connection.
        Ok(_) if state.panels.find_tab(tab).is_none() => state.dialogs.clear(),
        Ok(client) => {
            let Some((_, t)) = state.panels.find_tab_mut(tab) else {
                return;
            };
            let p = &mut t.panel;
            p.source = crate::fs::vfs::PanelSource::Remote(client);
            p.current_path = std::path::PathBuf::from("/");
            p.cursor_index = 0;
            p.clear_selection();
            state.dialogs.clear();
            state.refresh_tab(tab, show_hidden, true);
        }
        Err(e) => {
            state.dialogs.replace(PopupType::error(format!(
                "{} {}",
                crate::config::localization::t("error_ssh_failed"),
                e
            )));
        }
    }
    state.mark_ui_dirty();
}

/// Matches of the Find file search shown so far, and its end.
pub fn process_search_updates(state: &mut AppState) {
    let finished = state.search.poll();
    let running = state.search.is_running();
    let hits = match finished {
        Some(hits) => hits,
        None if running => match state.search.progress() {
            Some(hits) => hits,
            None => return,
        },
        None => return,
    };
    if let Some(PopupType::SearchResults(SearchResults {
        results, searching, ..
    })) = state.dialogs.top_mut()
        && (results.len() != hits.len() || !running)
    {
        *results = hits;
        *searching = running;
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
