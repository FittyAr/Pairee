use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Row, Table},
};

pub fn render_popup(f: &mut Frame, state: &AppState, context: &AppContext) {
    let popup = match &state.active_popup {
        Some(p) => p,
        None => return,
    };

    let theme = &context.config.theme;
    let size = f.size();

    match popup {
        PopupType::Help => {
            let area = centered_rect(60, 50, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(" Help - Keybindings ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let help_rows = vec![
                Row::new(vec!["Tab", "Switch active panel"]),
                Row::new(vec!["Insert / Space", "Tag/Select file for bulk ops"]),
                Row::new(vec!["F3", "View highlighted file contents"]),
                Row::new(vec!["F4", "Edit highlighted file"]),
                Row::new(vec!["F5", "Copy tagged files to passive panel"]),
                Row::new(vec!["F6", "Rename/Move files to passive panel"]),
                Row::new(vec!["F7", "Make new directory"]),
                Row::new(vec!["F8", "Delete tagged files"]),
                Row::new(vec!["Ctrl+H", "Toggle hidden files"]),
                Row::new(vec!["Ctrl+U", "Swap left and right panels"]),
                Row::new(vec!["F10", "Quit application"]),
                Row::new(vec!["Esc", "Close popup / Clear input"]),
            ];

            let table = Table::new(
                help_rows,
                [Constraint::Percentage(40), Constraint::Percentage(60)],
            )
            .block(block)
            .header(
                Row::new(vec!["Key", "Description"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            );

            f.render_widget(table, area);
        }
        PopupType::MkDirPrompt { input } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(" Create Directory ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = format!("\nEnter directory name:\n\n > {}", input);
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::ConfirmDelete { paths } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(" Confirm Deletion ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = format!(
                "\nAre you sure you want to delete {} item(s)?\n\n[Enter] Confirm Deletion\n[Esc] Cancel",
                paths.len()
            );
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::CopyProgress {
            current_file,
            files_copied,
            total_files,
            bytes_copied,
            total_bytes,
        } => {
            let area = centered_rect(55, 30, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(" Copying Files ")
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

            let file_label = format!("File: {}", current_file);
            let paragraph =
                Paragraph::new(file_label).style(Style::default().fg(parse_color(&theme.popup_fg)));
            f.render_widget(paragraph, inner_chunks[1]);

            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(Color::Yellow).bg(Color::DarkGray))
                .percent(percent.min(100))
                .label(format!("{}%", percent.min(100)));
            f.render_widget(gauge, inner_chunks[2]);

            let size_label = format!(
                "Files: {} / {}  |  Bytes: {} MB / {} MB",
                files_copied,
                total_files,
                *bytes_copied / (1024 * 1024),
                *total_bytes / (1024 * 1024)
            );
            let size_paragraph =
                Paragraph::new(size_label).style(Style::default().fg(parse_color(&theme.popup_fg)));
            f.render_widget(size_paragraph, inner_chunks[3]);
        }
        PopupType::Error(message) => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(" Error Alert ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = format!("\n {}\n\n[Press Enter/Esc to Dismiss]", message);
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(Color::LightRed));

            f.render_widget(paragraph, area);
        }
    }
}

/// Helper utility to divide screen space and center popup rectangles.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
