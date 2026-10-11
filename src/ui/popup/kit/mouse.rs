//! Mouse targets for dialog controls. A control records the keys that reach
//! and use it from where the focus is now, so the mouse drives each dialog
//! through the key handling it already has.

use crate::app::form::FormLayout;
use crate::ui::hit::{self, HitTarget};
use crate::ui::text_width::display_width;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;

/// `count` presses of `forward` (or of `back` when negative).
pub fn presses(count: i64, forward: KeyCode, back: KeyCode) -> Vec<KeyEvent> {
    let code = if count >= 0 { forward } else { back };
    let key = if code == KeyCode::BackTab {
        KeyEvent::new(code, KeyModifiers::SHIFT)
    } else {
        KeyEvent::from(code)
    };
    vec![key; count.unsigned_abs() as usize]
}

/// Records the rows of a list drawn top down in `area`, from entry `first`
/// on, `count` entries in all, the cursor on entry `cursor`.
pub fn list_rows(area: Rect, first: usize, count: usize, cursor: usize) {
    focused_list_rows(area, (first, count, cursor), &[]);
}

/// [`list_rows`] for a list that needs `focus` pressed first.
pub fn focused_list_rows(
    area: Rect,
    (first, count, cursor): (usize, usize, usize),
    focus: &[KeyEvent],
) {
    for (y, index) in (area.y..area.bottom()).zip(first..count) {
        list_item(Rect::new(area.x, y, area.width, 1), (index, cursor), focus);
    }
}

/// Records list entry `index` drawn at `area` (one or more rows), the
/// cursor on entry `cursor`, the list focused with `focus`.
pub fn list_item(area: Rect, (index, cursor): (usize, usize), focus: &[KeyEvent]) {
    let steps = (index as i64 - cursor as i64).clamp(i64::from(i32::MIN), i64::from(i32::MAX));
    hit::register(
        area,
        HitTarget::PopupRow {
            steps: steps as i32,
            focus: focus.to_vec(),
        },
    );
}

/// Records a control at `area` that `keys` use.
pub fn keys(area: Rect, keys: Vec<KeyEvent>) {
    hit::register(area, HitTarget::PopupKeys(keys));
}

/// Cells of the buttons of a centered button bar (`kit::button_bar`) drawn
/// on the first row of `area`.
pub fn button_cells(area: Rect, labels: &[String]) -> Vec<Rect> {
    span_cells(area, labels, 2, true)
}

/// Cells of `labels` laid out on the first row of `area` with `gap` blank
/// columns between them, centered or from the left edge.
pub fn span_cells(area: Rect, labels: &[String], gap: u16, centered: bool) -> Vec<Rect> {
    let widths: Vec<u16> = labels.iter().map(|l| display_width(l) as u16).collect();
    let total: u16 = widths.iter().sum::<u16>() + gap * widths.len().saturating_sub(1) as u16;
    let mut x = area.x
        + if centered {
            area.width.saturating_sub(total) / 2
        } else {
            0
        };
    widths
        .into_iter()
        .map(|width| {
            let cell = Rect::new(x, area.y, width, 1).intersection(area);
            x += width + gap;
            cell
        })
        .collect()
}

/// Buttons drawn as `labels` (see [`span_cells`]); a click on button `i`
/// presses `keys[i]`.
pub fn span_buttons(
    area: Rect,
    labels: &[String],
    (gap, centered): (u16, bool),
    keys: Vec<Vec<KeyEvent>>,
) {
    for (cell, keys) in span_cells(area, labels, gap, centered)
        .into_iter()
        .zip(keys)
    {
        self::keys(cell, keys);
    }
}

/// A single key press.
pub fn key(code: KeyCode) -> Vec<KeyEvent> {
    vec![KeyEvent::from(code)]
}

/// A form dialog's layout and where its focus is.
#[derive(Debug, Clone, Copy)]
pub struct FormFocus {
    pub form: FormLayout,
    pub focus: usize,
}

impl FormFocus {
    pub fn new(form: FormLayout, focus: usize) -> Self {
        Self { form, focus }
    }

    /// Tab / Shift+Tab presses moving the focus to `row` the short way.
    pub fn keys_to(&self, row: usize) -> Vec<KeyEvent> {
        let rows = self.form.rows.max(1) as i64;
        let forward = (row as i64 - self.focus as i64).rem_euclid(rows);
        let count = if forward * 2 <= rows {
            forward
        } else {
            forward - rows
        };
        presses(count, KeyCode::Tab, KeyCode::BackTab)
    }

    fn then(&self, row: usize, key: KeyCode) -> Vec<KeyEvent> {
        let mut keys = self.keys_to(row);
        keys.push(KeyEvent::from(key));
        keys
    }

    /// Text field on `row`, drawn at `area` with its text from `text_x`.
    pub fn field(&self, area: Rect, row: usize, text_x: u16) {
        hit::register(
            area,
            HitTarget::PopupField {
                keys: self.keys_to(row),
                text_x,
            },
        );
    }

    /// Option row (checkbox, cycle) `row` at `area`: focus and Space.
    pub fn option(&self, area: Rect, row: usize) {
        keys(area, self.then(row, KeyCode::Char(' ')));
    }

    /// Row `row` at `area` that Enter acts on.
    pub fn activate(&self, area: Rect, row: usize) {
        keys(area, self.then(row, KeyCode::Enter));
    }

    /// The button bar drawn at `area` with `labels`, buttons from
    /// `form.first_button` on.
    pub fn buttons(&self, area: Rect, labels: &[String]) {
        for (i, cell) in button_cells(area, labels).into_iter().enumerate() {
            self.activate(cell, self.form.first_button + i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_moves_the_short_way_round() {
        let focus = FormFocus::new(FormLayout::new(6, 4), 0);
        assert_eq!(focus.keys_to(2), presses(2, KeyCode::Tab, KeyCode::BackTab));
        assert_eq!(
            focus.keys_to(5),
            presses(-1, KeyCode::Tab, KeyCode::BackTab)
        );
        assert!(focus.keys_to(0).is_empty());
        assert_eq!(focus.keys_to(5)[0].modifiers, KeyModifiers::SHIFT);
    }

    #[test]
    fn button_cells_follow_the_centered_bar() {
        let labels = ["[ OK ]".to_string(), "[ Cancel ]".to_string()];
        let cells = button_cells(Rect::new(0, 3, 40, 1), &labels);
        // 6 + 2 + 10 = 18 columns, centered in 40: from column 11.
        assert_eq!(cells[0], Rect::new(11, 3, 6, 1));
        assert_eq!(cells[1], Rect::new(19, 3, 10, 1));
    }
}
