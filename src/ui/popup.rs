use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PopupType};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Row, Table},
};

pub fn render_popup(
    f: &mut Frame,
    state: &AppState,
    context: &AppContext,
    left_rect: Rect,
    right_rect: Rect,
) {
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

            f.render_widget(block, area);
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
        PopupType::Info(message) => {
            let area = centered_rect(55, 30, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Information ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = format!("\n {}\n\n[Press Enter/Esc to Dismiss]", message);
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

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
            // Center over the correct panel's rectangle
            let panel_rect = match panel {
                ActivePanel::Left => left_rect,
                ActivePanel::Right => right_rect,
            };
            let area = centered_rect_in(35, 60, panel_rect);
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

            let panel_label = match panel {
                ActivePanel::Left => "Left",
                ActivePanel::Right => "Right",
            };
            let title = format!(" Select Drive ({}) ", panel_label);
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
        PopupType::RenMovPrompt {
            input,
            src_paths,
            dest_dir,
        } => {
            let area = centered_rect(60, 30, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Rename / Move ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let count = src_paths.len();
            let first_name = src_paths
                .first()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let src_label = if count == 1 {
                format!("Moving: {}", first_name)
            } else {
                format!("Moving: {} items", count)
            };

            let text = format!(
                "\n {}\n Destination: {}\n\n > {}\n\n [Enter] Confirm   [Esc] Cancel",
                src_label,
                dest_dir.to_string_lossy(),
                input
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::SearchPrompt { query, search_root } => {
            let area = centered_rect(55, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Search Files ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = format!(
                "\n Search in: {}\n\n > {}\n\n [Enter] Search   [Esc] Cancel",
                search_root.to_string_lossy(),
                query
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::SearchResults {
            query,
            results,
            cursor_idx,
        } => {
            let area = centered_rect(70, 60, size);
            f.render_widget(Clear, area);

            let title = format!(" Search Results: \"{}\" ({} found) ", query, results.len());
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title)
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if results.is_empty() {
                let paragraph = Paragraph::new("\n No files found.\n\n [Esc] Close")
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let list_height = inner.height.saturating_sub(2) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                for (i, path) in results.iter().enumerate().skip(scroll_start).take(list_height) {
                    let is_cursor = i == *cursor_idx;
                    let display = path.to_string_lossy().to_string();
                    let style = if is_cursor {
                        Style::default()
                            .bg(parse_color(&theme.selection_bg))
                            .fg(parse_color(&theme.selection_fg))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(parse_color(&theme.popup_fg))
                    };
                    lines.push(Line::from(Span::styled(format!(" {} ", display), style)));
                }

                let hint = Line::from(Span::styled(
                    " [Enter] Navigate to  [Esc] Close ",
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph = Paragraph::new(lines)
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
        }
        PopupType::InfoPanel { lines } => {
            let area = centered_rect(55, 55, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" File Information ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text_lines: Vec<Line> = lines
                .iter()
                .map(|l| Line::from(Span::raw(format!(" {}", l))))
                .collect();

            let paragraph = Paragraph::new(text_lines)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::TreeView {
            nodes,
            cursor_idx,
            panel: _,
        } => {
            let area = centered_rect(55, 70, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(" Directory Tree ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let list_height = inner.height.saturating_sub(2) as usize;
            let scroll_start = cursor_idx.saturating_sub(list_height / 2);
            let mut lines = Vec::new();

            for (i, node) in nodes.iter().enumerate().skip(scroll_start).take(list_height) {
                let is_cursor = i == *cursor_idx;
                let indent = "  ".repeat(node.depth);
                let prefix = if node.is_dir { "▶ " } else { "  " };
                let display = format!("{}{}{}", indent, prefix, node.name);
                let style = if is_cursor {
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD)
                } else if node.is_dir {
                    Style::default().fg(Color::LightBlue)
                } else {
                    Style::default().fg(parse_color(&theme.popup_fg))
                };
                lines.push(Line::from(Span::styled(display, style)));
            }

            let hint = Line::from(Span::styled(
                " [Enter] Navigate  [Esc] Close ",
                Style::default().fg(Color::DarkGray),
            ));
            lines.push(Line::from(""));
            lines.push(hint);

            let paragraph = Paragraph::new(lines)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));
            f.render_widget(paragraph, inner);
        }
    }
}

/// Centers a rectangle of `percent_x` × `percent_y` over the full screen.
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

/// Centers a rectangle of `percent_x` × `percent_y` within a given parent rectangle.
/// Used for panel-specific popups (e.g. DriveSelect).
fn centered_rect_in(percent_x: u16, percent_y: u16, parent: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(parent);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
