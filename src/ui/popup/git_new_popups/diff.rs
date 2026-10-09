use crate::config::theme::Theme;
use crate::ui::popup::centered_rect;
use crate::ui::popup::kit;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// Renders the unified diff viewer.
pub fn render_diff_view(
    f: &mut Frame,
    state: &crate::app::state::popup::GitDiffViewState,
    theme: &Theme,
    size: Rect,
) -> bool {
    let file_path = &state.file_path;
    let commit_hash = &state.commit_hash;
    let diff_content = &state.diff_content;
    let scroll_y = &state.scroll_y;
    {
        let area = centered_rect(80, 80, size);
        let title = if let Some(path) = file_path {
            crate::config::localization::t("git_diff_view_title").replace("{}", path)
        } else if let Some(hash) = commit_hash {
            crate::config::localization::t("git_diff_commit_title").replace("{}", hash)
        } else {
            crate::config::localization::t("git_diff_default_title")
        };
        let inner = kit::frame_in(
            f,
            area,
            kit::accent_title(title, Color::Cyan),
            kit::fg(Color::Cyan),
            theme,
        );

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(1), // hint
            ])
            .split(inner);

        let content_area = chunks[0];
        let hint_area = chunks[1];

        // Process lines and colors
        let height = content_area.height as usize;
        let lines: Vec<Line> = if diff_content.trim().is_empty() {
            vec![Line::from(Span::styled(
                format!("  {}", crate::config::localization::t("git_diff_empty")),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ))]
        } else {
            diff_content
                .lines()
                .skip(*scroll_y)
                .take(height)
                .map(|line| {
                    let style = if line.starts_with('+') && !line.starts_with("+++") {
                        Style::default().fg(Color::Green)
                    } else if line.starts_with('-') && !line.starts_with("---") {
                        Style::default().fg(Color::Red)
                    } else if line.starts_with("@@") {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(parse_color(&theme.popup_fg))
                    };
                    Line::from(Span::styled(line.to_string(), style))
                })
                .collect()
        };

        f.render_widget(Paragraph::new(lines), content_area);

        // Hint bar
        let hint = crate::config::localization::t("git_diff_view_hint");
        f.render_widget(
            Paragraph::new(Span::styled(hint, Style::default().fg(Color::Yellow))),
            hint_area,
        );

        true
    }
}
