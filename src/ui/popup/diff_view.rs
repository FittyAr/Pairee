//! Painting of the diff viewer shared by the Git diff and Compare files:
//! a framed box with optional summary lines, the diff (unified, or side by
//! side with line numbers) and a hint line.

use crate::app::state::popup::DiffViewer;
use crate::diff::{DiffKind, DiffLine, SideRow};
use crate::ui::popup::{centered_rect_in, kit};
use crate::ui::resolved_theme::ResolvedTheme;
use crate::ui::text_width::{display_width, truncate_to_width};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// Width of a line-number column in the side-by-side layout.
const NUMBER_WIDTH: usize = 5;

/// What a diff box shows.
pub struct DiffBox<'a> {
    pub title: String,
    /// Lines above the diff (summary, binary comparison...).
    pub summary: Vec<Line<'static>>,
    pub viewer: &'a DiffViewer,
    /// Shown instead of the diff when it has no lines.
    pub empty: String,
    pub hint: String,
}

impl DiffBox<'_> {
    pub fn render(self, f: &mut Frame, screen: ratatui::layout::Rect, theme: &ResolvedTheme) {
        let area = centered_rect_in(80, 80, screen);
        let inner = kit::frame_in(
            f,
            area,
            kit::accent_title(self.title, Color::Cyan),
            kit::fg(Color::Cyan),
            theme,
        );
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(self.summary.len() as u16),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(inner);
        f.render_widget(
            Paragraph::new(self.summary).style(theme.popup_fg()),
            chunks[0],
        );
        let body = chunks[1];
        let lines = if self.viewer.doc.is_empty() {
            vec![Line::from(Span::styled(
                format!("  {}", self.empty),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ))]
        } else {
            body_lines(
                self.viewer,
                body.height as usize,
                body.width as usize,
                theme,
            )
        };
        f.render_widget(Paragraph::new(lines), body);
        f.render_widget(
            Paragraph::new(Span::styled(self.hint, Style::default().fg(Color::Yellow))),
            chunks[2],
        );
    }
}

/// The `height` rows shown from the viewer's scroll position.
fn body_lines(
    viewer: &DiffViewer,
    height: usize,
    width: usize,
    theme: &ResolvedTheme,
) -> Vec<Line<'static>> {
    let doc = &viewer.doc;
    if !viewer.side_by_side {
        return doc
            .lines
            .iter()
            .skip(viewer.scroll)
            .take(height)
            .map(|line| {
                Line::from(Span::styled(
                    truncate_to_width(&line.unified(), width),
                    kind_style(line.kind, theme),
                ))
            })
            .collect();
    }
    let half = width.saturating_sub(1) / 2;
    doc.side_by_side()
        .into_iter()
        .skip(viewer.scroll)
        .take(height)
        .map(|row| match row {
            SideRow::Full(i) => {
                let line = &doc.lines[i];
                Line::from(Span::styled(
                    truncate_to_width(&line.text, width),
                    kind_style(line.kind, theme),
                ))
            }
            SideRow::Pair(left, right) => {
                let cell = |i: Option<usize>, old: bool| {
                    side_cell(i.map(|i| &doc.lines[i]), old, half, theme)
                };
                Line::from(vec![
                    cell(left, true),
                    Span::styled("│", Style::default().fg(Color::DarkGray)),
                    cell(right, false),
                ])
            }
        })
        .collect()
}

/// One half of a side-by-side row: line number and text, padded to `width`.
fn side_cell(
    line: Option<&DiffLine>,
    old: bool,
    width: usize,
    theme: &ResolvedTheme,
) -> Span<'static> {
    let Some(line) = line else {
        return Span::raw(" ".repeat(width));
    };
    let number = if old { line.old_no } else { line.new_no };
    let number = number.map_or_else(String::new, |n| n.to_string());
    let text = truncate_to_width(&format!("{number:>NUMBER_WIDTH$} {}", line.text), width);
    let pad = width.saturating_sub(display_width(&text));
    Span::styled(
        format!("{text}{}", " ".repeat(pad)),
        kind_style(line.kind, theme),
    )
}

fn kind_style(kind: DiffKind, theme: &ResolvedTheme) -> Style {
    match kind {
        DiffKind::Added => Style::default().fg(Color::Green),
        DiffKind::Removed => Style::default().fg(Color::Red),
        DiffKind::Hunk => Style::default().fg(Color::Cyan),
        DiffKind::Header | DiffKind::Context => Style::default().fg(theme.popup_fg),
    }
}
