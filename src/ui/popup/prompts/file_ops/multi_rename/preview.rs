//! Preview table of the multi-rename dialog: old name, new name, status.

use crate::app::state::popup::MultiRenameState as Dialog;
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::fs::multi_rename::PreviewRow;
use crate::ui::popup::kit;
use crate::ui::scrollbar::{
    self, ScrollTarget, ScrollTargetId, ScrollView, ScrollbarSurface, ScrollbarUiState,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Paragraph,
};

/// Column widths in percent: old name, new name, status.
const COLUMNS: [u16; 3] = [40, 40, 20];

pub(super) fn render(
    f: &mut Frame,
    dialog: &Dialog,
    theme: &Theme,
    area: Rect,
    hits: &ScrollbarUiState,
) {
    let normal = kit::popup_fg(theme);
    let header = normal.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    let height = area.height.saturating_sub(1) as usize;
    let start = dialog
        .scroll
        .min(dialog.preview.rows.len().saturating_sub(height));
    let visible: Vec<&PreviewRow> = dialog
        .preview
        .rows
        .iter()
        .skip(start)
        .take(height)
        .collect();

    let mut columns: [Vec<Line>; 3] = [
        vec![Line::styled(t("multi_rename_col_old"), header)],
        vec![Line::styled(t("multi_rename_col_new"), header)],
        vec![Line::styled(t("multi_rename_col_status"), header)],
    ];
    for row in visible {
        let (status, style) = status(row, normal);
        columns[0].push(Line::styled(row.old.clone(), style));
        columns[1].push(Line::styled(row.new.clone(), style));
        columns[2].push(Line::styled(status, style));
    }

    let cells = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(COLUMNS.map(Constraint::Percentage))
        .split(area);
    for (lines, cell) in columns.into_iter().zip(cells.iter()) {
        f.render_widget(Paragraph::new(lines).style(normal), *cell);
    }
    // Rows below the header: scrollbar on the right, wheel over the table.
    let body = Rect {
        y: area.y.saturating_add(1),
        height: area.height.saturating_sub(1),
        ..area
    };
    scrollbar::render_vertical_right(
        f,
        body,
        ScrollView {
            content_len: dialog.preview.rows.len(),
            viewport_len: height,
            offset: start,
        },
        theme,
        ScrollTarget {
            surface: ScrollbarSurface::Popup,
            hits: Some(hits),
            id: ScrollTargetId::MultiRenamePreview,
        },
    );
}

/// Status label and row style: red with the issue, dim when unchanged.
fn status(row: &PreviewRow, normal: Style) -> (String, Style) {
    match row.issue {
        Some(issue) => (t(issue.label_key()), kit::fg(Color::LightRed)),
        None if !row.is_changed() => (t("multi_rename_unchanged"), kit::fg(Color::DarkGray)),
        None => (t("multi_rename_ok"), normal),
    }
}
