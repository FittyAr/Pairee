//! Lazy reconnection of restored SSH tabs: a tab connects the first time
//! it is shown; if that fails it stays on the local home folder.

use crate::app::context::AppContext;
use crate::app::state::tabs::PendingRemote;
use crate::app::state::{ActivePanel, AppState, PopupType, TabId};
use crate::config::localization::t;
use crate::fs::ssh::SharedSshClient;

/// Starts the connection of a shown restored SSH tab (one at a time).
pub fn resume_pending(state: &mut AppState, context: &AppContext) {
    if state.ssh_connect.is_running() {
        return;
    }
    let Some((id, preset_name)) = next_shown_pending(state) else {
        return;
    };
    let preset = context
        .config
        .settings
        .ssh_presets
        .iter()
        .find(|p| p.name == preset_name)
        .cloned();
    match preset {
        Some(preset) => {
            if let Some(pending) = pending_mut(state, id) {
                pending.connecting = true;
            }
            crate::app::input_popup::ssh_connect::actions::start_connection(
                state, context, id, preset,
            );
        }
        None => {
            if let Some((_, tab)) = state.panels.find_tab_mut(id) {
                tab.pending_remote = None;
            }
            notify(
                state,
                t("session_ssh_preset_missing").replace("{}", &preset_name),
            );
        }
    }
}

/// The shown tab (focused side first, then the other visible side) whose
/// SSH connection has not been started yet.
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
        Some((tab.id, pending.preset.clone()))
    })
}

fn pending_mut(state: &mut AppState, id: TabId) -> Option<&mut PendingRemote> {
    let (_, tab) = state.panels.find_tab_mut(id)?;
    tab.pending_remote.as_mut()
}

/// Applies a finished connection of a restored tab. Returns `false` when
/// tab `id` was not waiting for one (a connection from the SSH dialog).
pub fn finish(
    state: &mut AppState,
    id: TabId,
    result: &anyhow::Result<SharedSshClient>,
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
        Ok(client) => {
            if let Some((_, tab)) = state.panels.find_tab_mut(id) {
                let panel = &mut tab.panel;
                panel.source = crate::fs::vfs::PanelSource::Remote(client.clone());
                panel.current_path = pending.path;
                panel.pending_focus = pending.cursor;
                panel.cursor_index = 0;
                panel.clear_selection();
            }
            state.refresh_tab(id, show_hidden, true);
        }
        Err(e) => notify(
            state,
            t("session_ssh_restore_failed")
                .replacen("{}", &pending.preset, 1)
                .replacen("{}", &e.to_string(), 1),
        ),
    }
    true
}

fn notify(state: &mut AppState, message: String) {
    state.dialogs.push(PopupType::Info(message));
    state.mark_ui_dirty();
}
