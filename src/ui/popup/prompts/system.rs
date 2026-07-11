use super::super::centered_rect_fixed;
use crate::app::state::{AdminOpKind, PopupType, SelectMode};
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::ConfirmRetryAsAdmin { op_kind, .. } => {
            let area = centered_rect_fixed(65, 8, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("prompt_sudo_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text_key = match op_kind {
                AdminOpKind::Delete => "prompt_sudo_delete_text",
                AdminOpKind::MkDir => "prompt_sudo_mkdir_text",
            };

            let text = t(text_key);
            let paragraph = Paragraph::new(text)
                .block(block)
                .wrap(ratatui::widgets::Wrap { trim: true })
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::CopyProgress {
            is_move,
            current_file,
            files_copied,
            total_files,
            bytes_copied,
            total_bytes,
        } => {
            let area = centered_rect_fixed(55, 10, size);
            f.render_widget(Clear, area);

            let title = if *is_move {
                t("progress_move_title")
            } else {
                t("progress_copy_title")
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(title)
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let percent = bytes_copied
                .checked_mul(100)
                .and_then(|v| v.checked_div(*total_bytes))
                .unwrap_or(0) as u16;

            let inner_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Spacer
                    Constraint::Length(2), // File labels
                    Constraint::Length(3), // Progress bar
                    Constraint::Min(1),    // Size counts
                ])
                .split(block.inner(area));

            let file_label = t("progress_file_label").replacen("{}", current_file, 1);
            let paragraph =
                Paragraph::new(file_label).style(Style::default().fg(parse_color(&theme.popup_fg)));
            f.render_widget(paragraph, inner_chunks[1]);

            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(Color::Yellow).bg(Color::DarkGray))
                .percent(percent.min(100))
                .label(format!("{}%", percent.min(100)));
            f.render_widget(gauge, inner_chunks[2]);

            let size_label = t("progress_size_label")
                .replacen("{}", &files_copied.to_string(), 1)
                .replacen("{}", &total_files.to_string(), 1)
                .replacen("{}", &(*bytes_copied / (1024 * 1024)).to_string(), 1)
                .replacen("{}", &(*total_bytes / (1024 * 1024)).to_string(), 1);
            let size_paragraph =
                Paragraph::new(size_label).style(Style::default().fg(parse_color(&theme.popup_fg)));
            f.render_widget(size_paragraph, inner_chunks[3]);

            f.render_widget(block, area);
            true
        }
        PopupType::Error(message) => {
            let area = centered_rect_fixed(50, 8, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(t("prompt_error_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = format!("\n {}\n\n{}", message, t("prompt_dismiss_hint"));
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(Color::LightRed));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::Info(message) => {
            let area = centered_rect_fixed(55, 9, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("prompt_info_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = format!("\n {}\n\n{}", message, t("prompt_dismiss_hint"));
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::ApplyCommandPrompt { input, targets } => {
            let area = centered_rect_fixed(65, 10, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("prompt_apply_cmd_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let first_targets = targets
                .iter()
                .take(3)
                .map(|p| {
                    p.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                })
                .collect::<Vec<String>>()
                .join(", ");
            let files_label = if targets.len() > 3 {
                t("prompt_apply_cmd_plur")
                    .replacen("{}", &targets.len().to_string(), 1)
                    .replacen("{}", &first_targets, 1)
            } else {
                t("prompt_apply_cmd_sing").replacen("{}", &first_targets, 1)
            };

            let text = t("prompt_apply_cmd_text")
                .replacen("{}", &files_label, 1)
                .replacen("{}", input, 1);

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        PopupType::SelectGroupPrompt { mode, query } => {
            let area = centered_rect_fixed(50, 9, size);
            f.render_widget(Clear, area);

            let title = match mode {
                SelectMode::Add => t("prompt_select_group_title"),
                SelectMode::Remove => t("prompt_unselect_group_title"),
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title)
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let prompt_label = match mode {
                SelectMode::Add => t("prompt_select_group_pat"),
                SelectMode::Remove => t("prompt_unselect_group_pat"),
            };

            let text = format!(
                "\n {}\n\n > {}\n\n {}",
                prompt_label,
                query,
                t("prompt_confirm_cancel_hint")
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
            true
        }
        _ => false,
    }
}
