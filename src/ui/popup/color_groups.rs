//! "Color groups" and "Files highlighting" dialogs: a scrolled list of
//! `name < color >` rows with the cursor row highlighted.

use super::centered_rect_in;
use crate::app::state::PopupType;
use crate::app::text_input::TextField;
use crate::config::localization::t;
use crate::config::theme::COLOR_PROPS;
use crate::ui::popup::kit;
use crate::ui::resolved_theme::ResolvedTheme;
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

pub fn render_color_groups_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &ResolvedTheme,
    size: Rect,
) -> bool {
    let selection = theme.selection().add_modifier(Modifier::BOLD);
    let (title, cursor, edit, rows) = match popup {
        PopupType::ColorGroupsDialog {
            cursor_idx,
            edit,
            theme: edited,
        } => {
            let rows = COLOR_PROPS
                .iter()
                .map(|prop| ColorRow {
                    label: format!("{:<20}", prop.name),
                    color: (prop.get)(edited).clone(),
                    style: theme.popup_fg(),
                    selected: selection,
                })
                .collect::<Vec<_>>();
            (t("color_groups_title"), *cursor_idx, edit, rows)
        }
        PopupType::FilesHighlightingDialog {
            cursor_idx,
            edit,
            rules,
        } => {
            // Each rule is shown in its own color.
            let rows = rules
                .iter()
                .map(|rule| {
                    let fg = parse_color(&rule.color);
                    ColorRow {
                        label: format!("{:<30}", rule.mask),
                        color: rule.color.clone(),
                        style: Style::default().bg(theme.popup_bg).fg(fg),
                        selected: Style::default()
                            .bg(theme.selection_bg)
                            .fg(fg)
                            .add_modifier(Modifier::BOLD),
                    }
                })
                .collect::<Vec<_>>();
            (t("files_highlighting_title"), *cursor_idx, edit, rows)
        }
        _ => return false,
    };
    let border = kit::fg(theme.popup_border);
    let inner = kit::frame_in(f, centered_rect_in(60, 60, size), title, border, theme);
    let lines = color_lines(&rows, cursor, edit.as_ref(), inner.height as usize);
    f.render_widget(Paragraph::new(lines), inner);
    register_rows(inner, &rows, cursor, edit.is_some());
    true
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
            let value = match edit {
                Some(field) if focused => format!("{}_", field.text()),
                _ => row.color.clone(),
            };
            let style = if focused { row.selected } else { row.style };
            Line::from(Span::styled(
                format!(" {} < {:^15} >", row.label, value),
                style,
            ))
        })
        .collect()
}
