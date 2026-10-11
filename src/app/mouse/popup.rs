//! Mouse on dialog controls. Each control recorded the keys that reach it
//! (see `ui::popup::kit::mouse`); a click presses them through the dialog's
//! own key handling.

use super::Click;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::terminal::TerminalBackend;
use crate::ui::hit::{HitTarget, WHEEL_LINES};
use crate::ui::popup::kit::mouse::presses;
use crossterm::event::{KeyCode, KeyEvent, MouseButton};
use std::mem::Discriminant;

/// The dialog keys go to: stack depth and dialog kind.
fn identity(state: &AppState) -> Option<(usize, Discriminant<PopupType>)> {
    state
        .dialogs
        .top()
        .map(|popup| (state.dialogs.depth(), std::mem::discriminant(popup)))
}

/// Presses `keys` in order on the open dialog. Stops as soon as the dialog
/// closes or another one takes its place (the rest was meant for it).
pub async fn press(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    keys: impl IntoIterator<Item = KeyEvent>,
) -> anyhow::Result<()> {
    let dialog = identity(state);
    for key in keys {
        if identity(state) != dialog {
            break;
        }
        crate::app::app::events::dispatch_popup_key(state, context, terminal_backend, key).await?;
    }
    Ok(())
}

/// Acts on a press on a dialog control. Returns whether it was one.
pub async fn click(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    target: &HitTarget,
    click: Click,
) -> anyhow::Result<bool> {
    if click.button != MouseButton::Left {
        return Ok(false);
    }
    let keys = match target {
        HitTarget::PopupKeys(keys) => keys.clone(),
        HitTarget::PopupRow { steps } => {
            let mut keys = presses(i64::from(*steps), KeyCode::Down, KeyCode::Up);
            if click.is_double() {
                keys.push(KeyEvent::from(KeyCode::Enter));
            }
            keys
        }
        HitTarget::PopupField { keys, text_x } => {
            let mut keys = keys.clone();
            keys.push(KeyEvent::from(KeyCode::Home));
            let column = i64::from(click.column.saturating_sub(*text_x));
            keys.extend(presses(column, KeyCode::Right, KeyCode::Left));
            keys
        }
        _ => return Ok(false),
    };
    press(state, context, terminal_backend, keys).await?;
    Ok(true)
}

/// The wheel over a dialog list moves its cursor.
pub async fn wheel(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    down: bool,
) -> anyhow::Result<bool> {
    let steps = WHEEL_LINES as i64;
    let steps = if down { steps } else { -steps };
    press(
        state,
        context,
        terminal_backend,
        presses(steps, KeyCode::Down, KeyCode::Up),
    )
    .await?;
    Ok(true)
}
