//! "Color groups" and "Files highlighting" dialogs: a scrolled list of
//! `name < color >` rows with the cursor row highlighted.

use super::centered_rect;
use crate::app::state::PopupType;
use crate::app::text_input::TextField;
use crate::config::localization::t;
use crate::config::theme::{COLOR_PROPS, Theme};
use crate::ui::popup::kit;
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
    theme: &Theme,
    size: Rect,
) -> bool {
    let selection = kit::selection(theme).add_modifier(Modifier::BOLD);
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
                    style: kit::popup_fg(theme),
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
                        style: Style::default().bg(parse_color(&theme.popup_bg)).fg(fg),
                        selected: Style::default()
                            .bg(parse_color(&theme.selection_bg))
                            .fg(fg)
                            .add_modifier(Modifier::BOLD),
                    }
                })
                .collect::<Vec<_>>();
            (t("files_highlighting_title"), *cursor_idx, edit, rows)
        }
        _ => return false,
    };
    let border = kit::fg(parse_color(&theme.popup_border));
    let inner = kit::frame_in(f, centered_rect(60, 60, size), title, border, theme);
    let lines = color_lines(&rows, cursor, edit.as_ref(), inner.height as usize);
    f.render_widget(Paragraph::new(lines), inner);
    true
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
