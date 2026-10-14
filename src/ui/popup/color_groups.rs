//! "Color groups" and "Files highlighting" dialogs: a scrolled list of
//! `name < color >` rows with the cursor row highlighted.

use super::centered_rect_in;
use crate::app::state::popup::{ColorGroupsDialog, FilesHighlightingDialog};
use crate::app::text_input::TextField;
use crate::config::localization::t;
use crate::config::theme::COLOR_PROPS;
use crate::ui::popup::kit;
use crate::ui::popup::view::{DialogView, ViewCtx};
use crate::ui::text_width::{display_width, pad_to_width};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// One row: label, stored color, and its unfocused / focused styles.
struct ColorRow {
    label: String,
    color: String,
    style: Style,
    selected: Style,
}

impl DialogView for ColorGroupsDialog {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let theme = cx.theme;
        let selection = theme.selection().add_modifier(Modifier::BOLD);
        let rows = COLOR_PROPS
            .iter()
            .map(|prop| ColorRow {
                label: format!("{:<20}", prop.name),
                color: (prop.get)(&self.theme).clone(),
                style: theme.popup_fg(),
                selected: selection,
            })
            .collect::<Vec<_>>();
        let list = ColorList {
            title: t("color_groups_title"),
            cursor: self.cursor_idx,
            edit: self.edit.as_ref(),
            rows,
        };
        render_color_list(f, cx, &list);
    }
}

impl DialogView for FilesHighlightingDialog {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let theme = cx.theme;
        // Each rule is shown in its own color.
        let rows = self
            .rules
            .iter()
            .map(|rule| {
                let fg = parse_color(&rule.color);
                ColorRow {
                    label: pad_to_width(&rule.mask, 30),
                    color: rule.color.clone(),
                    style: Style::default().bg(theme.popup_bg).fg(fg),
                    selected: Style::default()
                        .bg(theme.selection_bg)
                        .fg(fg)
                        .add_modifier(Modifier::BOLD),
                }
            })
            .collect::<Vec<_>>();
        let list = ColorList {
            title: t("files_highlighting_title"),
            cursor: self.cursor_idx,
            edit: self.edit.as_ref(),
            rows,
        };
        render_color_list(f, cx, &list);
    }
}

/// What both dialogs show: title, rows, cursor and the color being typed.
struct ColorList<'a> {
    title: String,
    cursor: usize,
    edit: Option<&'a TextField>,
    rows: Vec<ColorRow>,
}

fn render_color_list(f: &mut Frame, cx: &ViewCtx<'_>, list: &ColorList<'_>) {
    let border = kit::fg(cx.theme.popup_border);
    let area = centered_rect_in(60, 60, cx.area);
    let inner = kit::frame_in(f, area, list.title.clone(), border, cx.theme);
    let lines = color_lines(&list.rows, list.cursor, list.edit, inner.height as usize);
    f.render_widget(Paragraph::new(lines), inner);
    register_rows(inner, &list.rows, list.cursor, list.edit.is_some());
}

/// Rows take clicks (double click edits); the `<` and `>` around a color
/// cycle it. While a color is typed, a click elsewhere commits it first.
fn register_rows(inner: Rect, rows: &[ColorRow], cursor: usize, editing: bool) {
    use crate::ui::popup::kit::mouse;
    use crossterm::event::{KeyCode, KeyEvent};
    let focus = if editing {
        mouse::key(KeyCode::Enter)
    } else {
        Vec::new()
    };
    let start = cursor.saturating_sub(inner.height as usize / 2);
    for (y, (i, row)) in (inner.y..inner.bottom()).zip(rows.iter().enumerate().skip(start)) {
        mouse::list_item(Rect::new(inner.x, y, inner.width, 1), (i, cursor), &focus);
        let left = inner.x + 1 + crate::ui::text_width::display_width(&row.label) as u16 + 1;
        for (x, code) in [(left, KeyCode::Left), (left + 18, KeyCode::Right)] {
            let mut keys = focus.clone();
            keys.extend(mouse::presses(
                i as i64 - cursor as i64,
                KeyCode::Down,
                KeyCode::Up,
            ));
            keys.push(KeyEvent::from(code));
            mouse::keys(Rect::new(x, y, 1, 1).intersection(inner), keys);
        }
    }
}

/// The edit cursor over a row drawn in `style`: its colors swapped.
fn cursor_style(style: Style) -> Style {
    style.add_modifier(ratatui::style::Modifier::REVERSED)
}

/// Columns of the value between the arrows.
const VALUE_WIDTH: usize = 15;

/// The visible rows, scrolled so the cursor stays near the middle.
fn color_lines(
    rows: &[ColorRow],
    cursor: usize,
    edit: Option<&TextField>,
    height: usize,
) -> Vec<Line<'static>> {
    let start = cursor.saturating_sub(height / 2);
    rows.iter()
        .enumerate()
        .skip(start)
        .take(height)
        .map(|(i, row)| {
            let focused = i == cursor;
            let style = if focused { row.selected } else { row.style };
            let value = match edit {
                // The value being typed, with its real cursor.
                Some(field) if focused => {
                    kit::FieldView::new(field, VALUE_WIDTH).spans(style, cursor_style(style), true)
                }
                _ => vec![Span::styled(row.color.clone(), style)],
            };
            // Centered in VALUE_WIDTH columns, as `{:^15}` would.
            let width: usize = value.iter().map(|s| display_width(&s.content)).sum();
            let pad = VALUE_WIDTH.saturating_sub(width);
            let mut spans = vec![Span::styled(
                format!(" {} < {}", row.label, " ".repeat(pad / 2)),
                style,
            )];
            spans.extend(value);
            spans.push(Span::styled(
                format!("{} >", " ".repeat(pad - pad / 2)),
                style,
            ));
            Line::from(spans)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_edited_value_shows_its_real_cursor() {
        let row = ColorRow {
            label: "Panel".into(),
            color: "Blue".into(),
            style: Style::default(),
            selected: Style::default(),
        };
        let mut field = TextField::new("Red");
        field.set_cursor(0);
        let lines = color_lines(std::slice::from_ref(&row), 0, Some(&field), 5);
        let cursor = lines[0]
            .spans
            .iter()
            .find(|s| s.style == cursor_style(row.selected))
            .expect("a cursor cell");
        assert_eq!(cursor.content, "R", "the cursor sits on the first letter");
        // Still centered in the same columns as a plain value.
        let plain = color_lines(std::slice::from_ref(&row), 0, None, 5);
        assert_eq!(lines[0].width(), plain[0].width());
    }
}
