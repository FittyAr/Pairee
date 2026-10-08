//! Maps cursor keys to editor motions: plain keys move, `Shift` extends a
//! stream selection and `Alt+Shift` (also with `Ctrl`) a vertical block
//! selection (Far style). In block mode (`Ctrl+B`) plain `Shift` selects a
//! block too, for terminals that intercept `Alt+Shift`+arrows.

use crate::app::editor::{Extend, Motion};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Motion and selection mode for `key`, or `None` when it does not move.
pub fn from_key(key: &KeyEvent, page: usize, block_mode: bool) -> Option<(Motion, Extend)> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let motion = match key.code {
        KeyCode::Up => Motion::Up(1),
        KeyCode::Down => Motion::Down(1),
        KeyCode::PageUp => Motion::Up(page),
        KeyCode::PageDown => Motion::Down(page),
        KeyCode::Left => Motion::Left,
        KeyCode::Right => Motion::Right,
        KeyCode::Home if ctrl => Motion::DocStart,
        KeyCode::End if ctrl => Motion::DocEnd,
        KeyCode::Home => Motion::LineStart,
        KeyCode::End => Motion::LineEnd,
        _ => return None,
    };
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let extend = match (shift, alt) {
        (true, true) => Extend::Block,
        (true, false) if block_mode => Extend::Block,
        (true, false) => Extend::Stream,
        _ => Extend::No,
    };
    Some((motion, extend))
}
