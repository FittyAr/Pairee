use crate::app::state::PopupType;
use crate::config::theme::Theme;
use crate::ui::popup::centered_rect_fixed;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Renders the git commit message input prompt.
pub fn render(f: &mut Frame, popup: &PopupType, theme: &Theme, size: Rect) -> bool {
    if let PopupType::GitCommitPrompt {
        input, cursor_idx, ..
    } = popup
    {
        let area = centered_rect_fixed(60, 7, size);
        f.render_widget(Clear, area);

        let title = format!(
            " {} ",
            crate::config::localization::t("git_commit_prompt_title")
        );
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // label
                Constraint::Length(1), // separator
                Constraint::Length(1), // input line
                Constraint::Length(1), // empty
                Constraint::Length(1), // hint
            ])
            .split(inner);

        f.render_widget(
            Paragraph::new(crate::config::localization::t("git_commit_msg_label"))
                .style(Style::default().fg(parse_color(&theme.popup_fg))),
            chunks[0],
        );

        f.render_widget(
            Paragraph::new("─".repeat(inner.width as usize))
                .style(Style::default().fg(Color::DarkGray)),
            chunks[1],
        );

        // Render input with a cursor indicator
        let before_cursor = &input[..*cursor_idx];
        let at_cursor = if *cursor_idx < input.len() {
            input[*cursor_idx..*cursor_idx + 1].to_string()
        } else {
            " ".to_string()
        };
        let after_cursor = if *cursor_idx < input.len() {
            input[*cursor_idx + 1..].to_string()
        } else {
            String::new()
        };

        let input_line = Line::from(vec![
            Span::styled(
                before_cursor.to_string(),
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
            Span::styled(
                at_cursor,
                Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.selection_fg)),
            ),
            Span::styled(
                after_cursor,
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ]);
        f.render_widget(Paragraph::new(input_line), chunks[2]);

        f.render_widget(
            Paragraph::new(crate::config::localization::t("git_commit_hint"))
                .style(Style::default().fg(Color::Yellow)),
            chunks[4],
        );

        true
    } else {
        false
    }
}
