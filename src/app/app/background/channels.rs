//! Background channel event processors for terminal, remote connections, search, and plugin dev tools.

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

/// Applies a finished connection from the connection dialog (or of a
/// restored tab, see `session::remote::finish`).
pub fn process_remote_connect_updates(state: &mut AppState, context: &AppContext) {
    let Some((tab, res)) = state.remote_connect.poll() else {
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
        Ok((conn, start)) => {
            let Some((_, t)) = state.panels.find_tab_mut(tab) else {
                return;
            };
            t.panel.open_remote(conn, start);
            state.dialogs.clear();
            state.refresh_tab(tab, show_hidden, true);
        }
        Err(e) => report_connect_failure(state, &e),
    }
    state.mark_ui_dirty();
}

/// Shows why a connection failed. The connection dialog it was started
/// from stays open under the message (the progress note above it closes),
/// so a wrong field can be fixed and tried again without reopening it.
fn report_connect_failure(state: &mut AppState, error: &anyhow::Error) {
    let message = PopupType::error(format!(
        "{} {}",
        crate::config::localization::t("error_remote_connect_failed"),
        error
    ));
    let from_dialog = state
        .dialogs
        .iter()
        .any(|d| matches!(d, PopupType::ConnectionPrompt(_)));
    if !from_dialog {
        state.dialogs.replace(message);
        return;
    }
    while !matches!(state.dialogs.top(), Some(PopupType::ConnectionPrompt(_))) {
        state.dialogs.pop();
    }
    state.dialogs.push(message);
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
    if let Some(SearchResults {
        results, searching, ..
    }) = state.dialogs.top_mut_as::<SearchResults>()
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
        let _ =
            crate::app::actions::exec::run_in_active_panel(state, &cmd, context, terminal_backend);
        state.refresh_both_panels(context.config.settings.show_hidden);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::{ActivePanel, ConnectionPromptState};
    use std::path::PathBuf;

    fn state() -> AppState {
        AppState::new(PathBuf::from("."), PathBuf::from("."))
    }

    #[test]
    fn a_failed_connection_keeps_its_dialog_open_under_the_error() {
        let mut state = state();
        state
            .dialogs
            .replace(PopupType::ConnectionPrompt(ConnectionPromptState::new(
                ActivePanel::Left,
                &[],
            )));
        state.dialogs.push(PopupType::info("connecting"));
        report_connect_failure(&mut state, &anyhow::anyhow!("no such host"));
        assert_eq!(state.dialogs.depth(), 2);
        assert!(matches!(state.dialogs.top(), Some(PopupType::Error(_))));
        state.dialogs.pop();
        assert!(matches!(
            state.dialogs.top(),
            Some(PopupType::ConnectionPrompt(_))
        ));
    }

    #[test]
    fn a_failed_restore_without_a_dialog_shows_only_the_error() {
        let mut state = state();
        report_connect_failure(&mut state, &anyhow::anyhow!("no such host"));
        assert_eq!(state.dialogs.depth(), 1);
        assert!(matches!(state.dialogs.top(), Some(PopupType::Error(_))));
    }
}
