//! Mouse on dialog controls. Each control recorded the keys that reach it
//! (see `ui::popup::kit::mouse`); a click presses them through the dialog's
//! own key handling.

use super::Click;
use super::dialog_drag::key_of;
use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::terminal::TerminalBackend;
use crate::ui::hit::{HitTarget, WHEEL_LINES};
use crate::ui::popup::kit::mouse::presses;
use crossterm::event::{KeyCode, KeyEvent, MouseButton};

/// Presses `keys` in order on the open dialog. Stops as soon as the dialog
/// closes or another one takes its place (the rest was meant for it).
pub async fn press(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    keys: impl IntoIterator<Item = KeyEvent>,
) -> anyhow::Result<()> {
    let dialog = key_of(&state.dialogs);
    for key in keys {
        if key_of(&state.dialogs) != dialog {
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
        HitTarget::PopupRow { steps, focus } => {
            let mut keys = focus.clone();
            if click.is_double() {
                // The first click already put the cursor on the row; the
                // list may have scrolled since, so the cell now shows
                // another row: open the one the cursor is on.
                keys.push(KeyEvent::from(KeyCode::Enter));
            } else {
                keys.extend(presses(i64::from(*steps), KeyCode::Down, KeyCode::Up));
            }
            keys
        }
        HitTarget::PopupField { keys, field } => {
            let mut keys = keys.clone();
            keys.push(KeyEvent::from(KeyCode::Home));
            let graphemes = field.graphemes_before(click.column) as i64;
            keys.extend(presses(graphemes, KeyCode::Right, KeyCode::Left));
            keys
        }
        _ => return Ok(false),
    };
    press(state, context, terminal_backend, keys).await?;
    Ok(true)
}

/// The wheel over a dialog list (focused with `focus`) moves its cursor.
pub async fn wheel(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    (focus, down): (Vec<KeyEvent>, bool),
) -> anyhow::Result<bool> {
    let steps = WHEEL_LINES as i64;
    let steps = if down { steps } else { -steps };
    let mut keys = focus;
    keys.extend(presses(steps, KeyCode::Down, KeyCode::Up));
    press(state, context, terminal_backend, keys).await?;
    Ok(true)
}
