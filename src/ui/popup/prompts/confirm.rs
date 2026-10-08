use super::super::centered_rect_fixed;
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
            let area = centered_rect_fixed(45, 7, size);
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
            let area = centered_rect_fixed(45, 7, size);
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
        PopupType::ConfirmReload => {
            let area = centered_rect_fixed(50, 8, size);
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
            let area = centered_rect_fixed(45, 7, size);
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
        PopupType::ConfirmUndo { direction, lines } => {
            render_undo(f, *direction, lines, theme, size);
            true
        }
        PopupType::SaveSetupConfirm => {
            let area = centered_rect_fixed(45, 7, size);
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

/// Undo/redo confirmation: question, entries and the Enter/Esc hint.
fn render_undo(
    f: &mut Frame,
    direction: crate::fs::journal::Direction,
    lines: &[String],
    theme: &crate::config::theme::Theme,
    size: Rect,
) {
    use crate::fs::journal::Direction;
    use crate::ui::popup::kit::{self, TextBox};
    let title = match direction {
        Direction::Undo => t("journal_undo_title"),
        Direction::Redo => t("journal_redo_title"),
    };
    let mut body: Vec<ratatui::text::Line> = lines
        .iter()
        .map(|line| ratatui::text::Line::from(format!(" {line}")))
        .collect();
    body.push(ratatui::text::Line::default());
    body.push(ratatui::text::Line::from(format!(
        " {}",
        t("journal_confirm_hint")
    )));
    let height = u16::try_from(body.len() + 2).unwrap_or(u16::MAX);
    TextBox {
        size: (70, height),
        title,
        border: kit::fg(Color::Yellow),
        body: body.into(),
        body_style: kit::popup_fg(theme),
    }
    .render(f, size, theme);
}
