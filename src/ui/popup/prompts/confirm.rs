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
        PopupType::ConfirmQuit => {
            let area = centered_rect(45, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("prompt_exit_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_exit_text");
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::ConfirmInterrupt => {
            let area = centered_rect(45, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(t("prompt_abort_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_abort_text");
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::ConfirmReload { .. } => {
            let area = centered_rect(50, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("prompt_reload_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_reload_text");
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::ConfirmClearHistory { history_type } => {
            let area = centered_rect(45, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("prompt_clear_history_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let hist_type_translated = match history_type.as_str() {
                "command" => t("history_type_command"),
                "view" => t("history_type_view"),
                "folder" => t("history_type_folder"),
                _ => history_type.clone(),
            };

            let text = t("prompt_clear_history_text").replacen("{}", &hist_type_translated, 1);
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::SaveSetupConfirm => {
            let area = centered_rect(45, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(t("prompt_save_setup_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("prompt_save_setup_text");
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        _ => false,
    }
}
