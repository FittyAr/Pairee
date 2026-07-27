use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::CopyProgress { .. }) = state.active_popup {
        if key.code == KeyCode::Esc {
            if context
                .config
                .settings
                .confirmations
                .confirm_interrupt_operation
            {
                state.active_popup = Some(PopupType::ConfirmInterrupt);
            } else {
                // A10: progress_rx is gone. The
                // unified engine handles cancel
                // through the job's is_cancelled
                // flag, not through the channel.
                state.active_popup = None;
                state.refresh_both_panels(context.config.settings.show_hidden);
            }
            return Ok(None);
        }
        Err(())
    } else {
        Err(())
    }
}
