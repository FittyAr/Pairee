//! Background update orchestrator for terminal, SSH, searches, transfers, and dev tools.

pub mod channels;
pub mod transfer;

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::terminal::TerminalBackend;

pub fn process_background_updates(
    state: &mut AppState,
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) {
    state.poll_panel_listings();
    state.poll_quick_view();
    state.poll_viewer();
    if state.disk_usage.poll() {
        state.mark_ui_dirty();
    }
    crate::app::git_ops::poll_git_op(state);
    crate::app::actions::fs_ops::multi_rename::poll(state, context);
    crate::app::sync::poll_folder_scan(state);
    channels::process_terminal_updates(state);
    channels::process_ssh_connect_updates(state, context);
    channels::process_search_updates(state);
    channels::process_dev_progress_updates(state);
    transfer::process_transfer_events(state, context);
    channels::process_pending_custom_command(state, context, terminal_backend);
}
