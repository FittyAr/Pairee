//! Lazy reconnection of restored remote tabs: a tab connects the first time
//! it is shown; if that fails it stays on the local home folder.

use crate::app::context::AppContext;
use crate::app::state::tabs::PendingRemote;
use crate::app::state::{ActivePanel, AppState, PopupType, TabId};
use crate::config::localization::t;

/// Starts the connection of a shown restored remote tab (one at a time).
pub fn resume_pending(state: &mut AppState, context: &AppContext) {
    if state.remote_connect.is_running() {
        return;
    }
    let Some((id, name)) = next_shown_pending(state) else {
        return;
    };
    let saved = context
        .config
        .settings
        .connections
        .iter()
        .find(|c| c.name == name)
        .cloned();
    match saved {
        Some(conn) => {
            if let Some(pending) = pending_mut(state, id) {
                pending.connecting = true;
            }
            crate::app::input_popup::connection::actions::start_connection(
                state, context, id, conn,
            );
        }
        None => {
            if let Some((_, tab)) = state.panels.find_tab_mut(id) {
                tab.pending_remote = None;
            }
            notify(state, t("session_connection_missing").replace("{}", &name));
        }
    }
}

/// The shown tab (focused side first, then the other visible side) whose
/// remote connection has not been started yet.
fn next_shown_pending(state: &AppState) -> Option<(TabId, String)> {
    let panels = &state.panels;
    let focused = panels.active;
    let other_visible = match focused.other() {
        ActivePanel::Left => panels.left_visible,
        ActivePanel::Right => panels.right_visible,
    };
    let sides = std::iter::once(focused).chain(other_visible.then(|| focused.other()));
    sides.into_iter().find_map(|side| {
        let tab = panels.tabs(side).active();
        let pending = tab.pending_remote.as_ref().filter(|p| !p.connecting)?;
        Some((tab.id, pending.connection.clone()))
    })
}

/// The running connection is being replaced: restored tabs that waited for
/// it connect again when next shown.
pub fn requeue_pending(state: &mut AppState) {
    for (_, tab) in state.panels.all_tabs_mut() {
        if let Some(pending) = tab.pending_remote.as_mut() {
            pending.connecting = false;
        }
    }
}

fn pending_mut(state: &mut AppState, id: TabId) -> Option<&mut PendingRemote> {
    let (_, tab) = state.panels.find_tab_mut(id)?;
    tab.pending_remote.as_mut()
}

/// Applies a finished connection of a restored tab. Returns `false` when
/// tab `id` was not waiting for one (a connection from the dialog).
pub fn finish(
    state: &mut AppState,
    id: TabId,
    result: &crate::app::state::RemoteConnectResult,
    show_hidden: bool,
) -> bool {
    let Some(pending) = state
        .panels
        .find_tab_mut(id)
        .and_then(|(_, tab)| tab.pending_remote.take())
    else {
        return false;
    };
    match result {
        Ok((conn, _)) => {
            if let Some((_, tab)) = state.panels.find_tab_mut(id) {
                let panel = &mut tab.panel;
                panel.source = crate::fs::vfs::PanelSource::Remote(conn.clone());
                panel.current_path = pending.path;
                panel.pending_focus = pending.cursor;
                panel.cursor_index = 0;
                panel.clear_selection();
                if pending.locked {
                    tab.lock_here();
                }
            }
            state.refresh_tab(id, show_hidden, true);
        }
        Err(e) => notify(
            state,
            t("session_remote_restore_failed")
                .replacen("{}", &pending.connection, 1)
                .replacen("{}", &e.to_string(), 1),
        ),
    }
    true
}

fn notify(state: &mut AppState, message: String) {
    state.dialogs.push(PopupType::info(message));
    state.mark_ui_dirty();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_replaced_connection_lets_the_restored_tab_try_again() {
        let mut state = AppState::new(".".into(), ".".into());
        let id = state.panels.active_tab_id(state.panels.active);
        state.panels.find_tab_mut(id).unwrap().1.pending_remote = Some(PendingRemote {
            connection: "srv".into(),
            path: "/home".into(),
            cursor: None,
            connecting: true,
            locked: false,
        });
        assert!(next_shown_pending(&state).is_none(), "waiting for its job");
        requeue_pending(&mut state);
        assert_eq!(next_shown_pending(&state), Some((id, "srv".to_string())));
    }
}
