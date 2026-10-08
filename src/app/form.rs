//! Key handling shared by every form-style dialog: a column of focusable rows
//! (an optional text field, option rows) followed by a button bar.
//!
//! Dialogs describe their rows with a [`FormLayout`] and react only to the
//! resulting [`FormKey`] (toggle a row, activate a row, cancel); focus
//! movement and text editing live here once.

use crate::app::list_nav::{wrap_next, wrap_prev};
use crate::app::text_input::TextField;
use crossterm::event::{KeyCode, KeyEvent};

/// Shape of a form dialog: `rows` focusable rows, the text field (if any) on
/// `input_row`, and the buttons on rows `first_button..rows`.
#[derive(Debug, Clone, Copy)]
pub struct FormLayout {
    pub rows: usize,
    pub input_row: Option<usize>,
    pub first_button: usize,
}

/// What a key means for a form dialog after focus / editing was applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormKey {
    /// Focus moved, text edited or key swallowed; nothing else to do.
    Handled,
    /// Space on an option row.
    Toggle(usize),
    /// Enter on any row (buttons included).
    Activate(usize),
    /// Esc.
    Cancel,
    /// Any other key (function keys...), left to the dialog.
    Other,
}

impl FormLayout {
    /// A text field on row 0 followed by buttons from `first_button`.
    pub const fn with_input(rows: usize, first_button: usize) -> Self {
        Self {
            rows,
            input_row: Some(0),
            first_button,
        }
    }

    pub fn is_button(&self, row: usize) -> bool {
        (self.first_button..self.rows).contains(&row)
    }

    /// Index of the focused button inside the button bar, if any.
    pub fn focused_button(&self, focus: usize) -> Option<usize> {
        self.is_button(focus).then(|| focus - self.first_button)
    }

    /// Applies `key` to the focus (`*focus`) and to `field` (the text field on
    /// [`Self::input_row`]).
    pub fn handle(
        &self,
        focus: &mut usize,
        field: Option<&mut TextField>,
        key: &KeyEvent,
    ) -> FormKey {
        let row = *focus;
        let on_input = self.input_row == Some(row);
        match key.code {
            KeyCode::Up | KeyCode::BackTab => *focus = wrap_prev(row, self.rows),
            KeyCode::Down | KeyCode::Tab => *focus = wrap_next(row, self.rows),
            KeyCode::Left | KeyCode::Right if self.is_button(row) => {
                let count = self.rows - self.first_button;
                let step = if key.code == KeyCode::Left {
                    wrap_prev
                } else {
                    wrap_next
                };
                *focus = self.first_button + step(row - self.first_button, count);
            }
            KeyCode::Enter => return FormKey::Activate(row),
            KeyCode::Esc => return FormKey::Cancel,
            KeyCode::Char(' ') if !on_input => return FormKey::Toggle(row),
            _ => {
                let edited = match field {
                    Some(field) if on_input => field.handle_key(key).consumed(),
                    _ => false,
                };
                if !edited && !is_editing_key(key.code) {
                    return FormKey::Other;
                }
            }
        }
        FormKey::Handled
    }
}

/// Keys a form swallows even when the focused row is not a text field.
fn is_editing_key(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::Char(_)
            | KeyCode::Backspace
            | KeyCode::Delete
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    const LAYOUT: FormLayout = FormLayout::with_input(4, 2);

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn focus_wraps_and_buttons_cycle() {
        let mut focus = 0;
        assert_eq!(
            LAYOUT.handle(&mut focus, None, &key(KeyCode::Up)),
            FormKey::Handled
        );
        assert_eq!(focus, 3);
        LAYOUT.handle(&mut focus, None, &key(KeyCode::Right));
        assert_eq!(focus, 2);
        assert_eq!(LAYOUT.focused_button(focus), Some(0));
        LAYOUT.handle(&mut focus, None, &key(KeyCode::Tab));
        assert_eq!(focus, 3);
        assert_eq!(
            LAYOUT.handle(&mut focus, None, &key(KeyCode::Enter)),
            FormKey::Activate(3)
        );
    }

    #[test]
    fn input_row_edits_and_option_rows_toggle() {
        let mut field = TextField::new("ab");
        let mut focus = 0;
        LAYOUT.handle(&mut focus, Some(&mut field), &key(KeyCode::Char(' ')));
        LAYOUT.handle(&mut focus, Some(&mut field), &key(KeyCode::Home));
        LAYOUT.handle(&mut focus, Some(&mut field), &key(KeyCode::Delete));
        assert_eq!(field.text(), "b ");
        focus = 1;
        assert_eq!(
            LAYOUT.handle(&mut focus, Some(&mut field), &key(KeyCode::Char(' '))),
            FormKey::Toggle(1)
        );
        assert_eq!(
            LAYOUT.handle(&mut focus, Some(&mut field), &key(KeyCode::F(10))),
            FormKey::Other
        );
        assert_eq!(
            LAYOUT.handle(&mut focus, Some(&mut field), &key(KeyCode::Esc)),
            FormKey::Cancel
        );
    }
}
