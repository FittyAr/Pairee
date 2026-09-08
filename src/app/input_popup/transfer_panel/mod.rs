//! Transfer panel popup keyboard handler.

mod conflict;
mod navigation;
mod queue;

use crate::app::context::AppContext;
use crate::app::state::{AppState, TransferTab, TransferViewMode};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let transfer = match &mut state.transfer {
        Some(t) => t,
        None => return Err(()),
    };

    if let Some((job_id, _, _)) = transfer.active_conflict_info {
        return conflict::handle_conflict(transfer, key, job_id);
    }

    match key.code {
        KeyCode::Char('t') | KeyCode::Char('T')
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
            Ok(Some(Action::ToggleTransferPanel))
        }
        KeyCode::Esc => {
            transfer.view_mode = TransferViewMode::Minimized;
            state.dialogs.clear();
            Ok(None)
        }
        _ if navigation::handle_navigation(transfer, key.code) => Ok(None),
        _ if queue::handle_queue_action(transfer, &mut state.dialogs, context, key.code) => {
            Ok(None)
        }
        KeyCode::Enter | KeyCode::Char(' ') if transfer.active_tab == TransferTab::Options => {
            navigation::handle_options_toggle(transfer);
            Ok(None)
        }
        _ => Err(()),
    }
}
