use super::super::centered_rect;
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::FilePanelFilterPrompt { input } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("prompt_filter_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_filter_text").replacen("{}", input, 1);

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::QuickFilterPrompt { input, .. } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("prompt_quick_filter_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_quick_filter_text").replacen("{}", input, 1);

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::CopyMoveFilterPrompt { input, previous: _ } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("prompt_filter_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_filter_text").replacen("{}", input, 1);

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        _ => false,
    }
}
