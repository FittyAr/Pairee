use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
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
        PopupType::UserMenu => {
            let area = centered_rect(50, 35, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(" User Commands Menu ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let menu_rows = vec![
                Row::new(vec!["1", "Refresh Panel Directories"]),
                Row::new(vec!["2", "Toggle Hidden Files"]),
                Row::new(vec!["3", "Swap Left and Right Panels"]),
                Row::new(vec!["4", "Show Help Keyboard Shortcuts"]),
                Row::new(vec!["5", "Close User Menu"]),
            ];

            let table = Table::new(
                menu_rows,
                [Constraint::Percentage(20), Constraint::Percentage(80)],
            )
            .block(block)
            .header(
                Row::new(vec!["Key", "Command"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            );

            f.render_widget(table, area);
        }
        PopupType::InternalEditor {
            path,
            lines,
            cursor_x,
            cursor_y,
            scroll_y,
            is_dirty,
        } => {
            let area = centered_rect(95, 90, size);
            f.render_widget(Clear, area);

            let title = format!(
                " Editor - {} {} ",
                path.to_string_lossy(),
                if *is_dirty { "*" } else { "" }
            );

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title)
                .style(Style::default().bg(Color::Blue));

            let height = area.height.saturating_sub(2) as usize;
            let visible_lines: Vec<String> =
                lines.iter().skip(*scroll_y).take(height).cloned().collect();

            let mut text = Vec::new();
            for line in visible_lines {
                text.push(ratatui::text::Line::from(line));
            }

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(Color::White));

            f.render_widget(paragraph, area);

            // Draw the terminal blinking cursor at the editing position
            let editor_cursor_x = area.x + 1 + *cursor_x as u16;
            let editor_cursor_y = area.y + 1 + (*cursor_y - *scroll_y) as u16;

            if editor_cursor_x < area.x + area.width - 1
                && editor_cursor_y < area.y + area.height - 1
            {
                f.set_cursor(editor_cursor_x, editor_cursor_y);
            }
        }
        PopupType::Menu {
            active_menu_idx,
            active_item_idx,
        } => {
            let items = crate::ui::menu::get_menu_items(*active_menu_idx);
            let dropdown_x = match active_menu_idx {
                0 => 2,
                1 => 10,
                2 => 19,
                3 => 31,
                4 => 42,
                _ => 2,
            };
            let dropdown_width = 30;
            let dropdown_height = (items.len() + 2) as u16;
            let dropdown_rect = Rect::new(dropdown_x, 1, dropdown_width, dropdown_height);

            f.render_widget(Clear, dropdown_rect);

            let mut lines = Vec::new();
            for (i, item) in items.iter().enumerate() {
                let is_cursor = i == *active_item_idx;
                let style = if is_cursor {
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(parse_color(&theme.popup_fg))
                };
                lines.push(Line::from(Span::styled(*item, style)));
            }

            let paragraph = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                    .style(Style::default().bg(parse_color(&theme.popup_bg))),
            );

            f.render_widget(paragraph, dropdown_rect);
        }
        PopupType::DriveSelect {
            panel,
            drives,
            cursor_idx,
        } => {
            let area = centered_rect(35, 35, size);
            f.render_widget(Clear, area);

            let mut lines = Vec::new();
            for (i, drive) in drives.iter().enumerate() {
                let is_cursor = i == *cursor_idx;
                let line_str = if is_cursor {
                    format!(" >  {} ", drive)
                } else {
                    format!("    {} ", drive)
                };
                let style = if is_cursor {
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(parse_color(&theme.popup_fg))
                };
                lines.push(Line::from(Span::styled(line_str, style)));
            }

            let title = format!(" Select Drive ({:?}) ", panel);
            let paragraph = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                    .title(title)
                    .style(Style::default().bg(parse_color(&theme.popup_bg))),
            );

            f.render_widget(paragraph, area);
        }
        PopupType::Hotlist {
            bookmarks,
            cursor_idx,
        } => {
            let area = centered_rect(60, 40, size);
            f.render_widget(Clear, area);

            let mut lines = Vec::new();
            for (i, (name, path)) in bookmarks.iter().enumerate() {
                let is_cursor = i == *cursor_idx;
                let line_str = format!(" {:<20} ->  {} ", name, path.to_string_lossy());
                let style = if is_cursor {
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(parse_color(&theme.popup_fg))
                };
                lines.push(Line::from(Span::styled(line_str, style)));
            }

            let paragraph = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                    .title(" Directory Hotlist ")
                    .style(Style::default().bg(parse_color(&theme.popup_bg))),
            );

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
