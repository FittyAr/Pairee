//! Progress popup for background Git network operations.

use super::super::centered_rect_fixed;
use crate::app::state::popup::GitProgress;
use crate::config::localization::t;
use crate::git::remote::TransferStats;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
};

impl DialogView for GitProgress {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let stats = cx.state.git_op.progress();
        render_git(f, &self.title, stats, cx.theme, cx.area);
    }
}

fn render_git(
    f: &mut Frame,
    title: &str,
    stats: Option<TransferStats>,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
) {
    let area = centered_rect_fixed(60, 7, size);
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title.to_string())
        .style(Style::default().bg(theme.popup_bg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    let (label, ratio) = match stats {
        Some(s) if s.total > 0 => (
            t("git_progress_objects")
                .replacen("{}", &s.done.to_string(), 1)
                .replacen("{}", &s.total.to_string(), 1)
                .replacen("{}", &bytesize::ByteSize::b(s.bytes as u64).to_string(), 1),
            (s.done as f64 / s.total as f64).clamp(0.0, 1.0),
        ),
        _ => (t("git_progress_waiting"), 0.0),
    };
    let fg = Style::default().fg(theme.popup_fg);
    f.render_widget(Paragraph::new(label).style(fg), rows[0]);
    f.render_widget(
        Gauge::default()
            .gauge_style(Style::default().fg(Color::Cyan))
            .ratio(ratio),
        rows[1],
    );
    f.render_widget(Paragraph::new(t("git_progress_hint")).style(fg), rows[3]);
}
