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

/// Renders the git checkout confirmation dialog.
pub fn render(f: &mut Frame, popup: &PopupType, theme: &Theme, size: Rect) -> bool {
    if let PopupType::GitConfirmCheckout {
        target, is_branch, ..
    } = popup
    {
        let area = centered_rect_fixed(60, 8, size);
        f.render_widget(Clear, area);

        let kind_str = if *is_branch {
            crate::config::localization::t("git_checkout_branch")
        } else {
            crate::config::localization::t("git_checkout_commit")
        };

        let title = format!(" {} ", crate::config::localization::t("git_checkout_title"));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let msg1 = format!(
            "  {} {}:",
            crate::config::localization::t("git_checkout_confirm"),
            kind_str
        );
        f.render_widget(
            Paragraph::new(msg1).style(Style::default().fg(parse_color(&theme.popup_fg))),
            chunks[0],
        );
        f.render_widget(
            Paragraph::new(format!("  {}", target)).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            chunks[1],
        );

        f.render_widget(
            Paragraph::new("─".repeat(inner.width as usize))
                .style(Style::default().fg(Color::DarkGray)),
            chunks[2],
        );

        let buttons = Line::from(vec![
            Span::styled(
                "  [ Enter / Y ] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                crate::config::localization::t("git_checkout_yes"),
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
            Span::styled("    ", Style::default()),
            Span::styled(
                "[ Esc / N ] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                crate::config::localization::t("git_checkout_no"),
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ]);
        f.render_widget(Paragraph::new(buttons), chunks[3]);

        true
    } else {
        false
    }
}
