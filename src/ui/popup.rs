use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PopupType, LinkKind, SelectMode, SortField, CompareStatus};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Row, Table},
};

fn render_editor_widget(
    f: &mut Frame,
    area: Rect,
    path: &std::path::Path,
    lines: &[String],
    cursor_x: usize,
    cursor_y: usize,
    scroll_y: usize,
    is_dirty: bool,
    _theme: &crate::config::theme::Theme,
) {
    let title = format!(
        " Editor - {} {} ",
        path.to_string_lossy(),
        if is_dirty { "*" } else { "" }
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title)
        .style(Style::default().bg(Color::Blue));

    let inner = block.inner(area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);
    let edit_area = chunks[0];
    let status_area = chunks[1];

    let height = edit_area.height as usize;
    let visible_lines: Vec<String> =
        lines.iter().skip(scroll_y).take(height).cloned().collect();

    let mut text = Vec::new();
    for (idx, line) in visible_lines.into_iter().enumerate() {
        let line_num = scroll_y + idx + 1;
        let prefix = format!("{:>4} │ ", line_num);
        text.push(ratatui::text::Line::from(format!("{}{}", prefix, line)));
    }

    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(Color::White));

    f.render_widget(block, area);
    f.render_widget(paragraph, edit_area);

    let current_line_len = lines.get(cursor_y).map(|l| l.len()).unwrap_or(0);
    let status_text = format!(
        " Line Chars: {} | Total Lines: {} | Pos: ({}, {})",
        current_line_len,
        lines.len(),
        cursor_y + 1,
        cursor_x + 1
    );
    let status_para = Paragraph::new(status_text)
        .style(Style::default().bg(Color::Cyan).fg(Color::Black));
    f.render_widget(status_para, status_area);

    // Draw the terminal blinking cursor at the editing position
    let prefix_len = 7u16;
    let editor_cursor_x = edit_area.x + prefix_len + cursor_x as u16;
    let editor_cursor_y = edit_area.y + (cursor_y - scroll_y) as u16;

    if editor_cursor_x < edit_area.x + edit_area.width
        && editor_cursor_y < edit_area.y + edit_area.height
    {
        f.set_cursor(editor_cursor_x, editor_cursor_y);
    }
}

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
                Row::new(vec!["6", "Download 7z Extractor Tool"]),
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
            last_search: _,
        } => {
            let area = centered_rect(95, 90, size);
            f.render_widget(Clear, area);
            render_editor_widget(f, area, path, lines, *cursor_x, *cursor_y, *scroll_y, *is_dirty, theme);
        }
        PopupType::EditorSearchPrompt {
            path,
            lines,
            cursor_x,
            cursor_y,
            scroll_y,
            is_dirty,
            last_search: _,
            query,
        } => {
            let area = centered_rect(95, 90, size);
            f.render_widget(Clear, area);
            render_editor_widget(f, area, path, lines, *cursor_x, *cursor_y, *scroll_y, *is_dirty, theme);

            // Overlay search input popup
            let search_area = centered_rect(50, 15, size);
            f.render_widget(Clear, search_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Search Text ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = format!(
                "\n Search query:\n > {}\n\n [Enter] Search   [Esc] Cancel",
                query
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, search_area);
        }
        PopupType::InternalViewer { viewer } => {
            let area = centered_rect(95, 90, size);
            f.render_widget(Clear, area);
            crate::ui::viewer::render_viewer(f, area, viewer, theme);
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
        PopupType::SearchPrompt { query, content_query, search_root, focus_content } => {
            let area = centered_rect(55, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Search Files ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let q_pref = if !*focus_content { "► " } else { "  " };
            let c_pref = if *focus_content { "► " } else { "  " };

            let text = format!(
                "\n Search in: {}\n{}File name query: {}\n{}Content query: {}\n\n [Tab] Switch Field   [Enter] Search   [Esc] Cancel",
                search_root.to_string_lossy(),
                q_pref,
                query,
                c_pref,
                content_query
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
        PopupType::ContextMenu { items, cursor_idx } => {
            let panel_rect = match state.active_panel {
                ActivePanel::Left => left_rect,
                ActivePanel::Right => right_rect,
            };
            let height_percent = std::cmp::min(100, std::cmp::max(20, (items.len() * 10) as u16));
            let area = centered_rect_in(50, height_percent, panel_rect);
            f.render_widget(Clear, area);

            let mut lines = Vec::new();
            for (i, item) in items.iter().enumerate() {
                let is_cursor = i == *cursor_idx;
                let line_str = if is_cursor {
                    format!(" >  {} ", item)
                } else {
                    format!("    {} ", item)
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

            let paragraph = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                    .title(" Actions ")
                    .style(Style::default().bg(parse_color(&theme.popup_bg))),
            );
            f.render_widget(paragraph, area);
        }
        PopupType::CompressPrompt {
            input,
            targets,
            dest_dir,
        } => {
            let area = centered_rect(60, 30, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Compress Archive ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let count = targets.len();
            let first_name = targets
                .first()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let src_label = if count == 1 {
                format!("Compressing: {}", first_name)
            } else {
                format!("Compressing: {} items", count)
            };

            let text = format!(
                "\n {}\n Destination: {}\n\n > {}.zip\n\n [Enter] Confirm   [Esc] Cancel",
                src_label,
                dest_dir.to_string_lossy(),
                input
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::ApplyCommandPrompt { input, targets } => {
            let area = centered_rect(65, 35, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Apply Command to Selected Files ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let first_targets = targets
                .iter()
                .take(3)
                .map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())
                .collect::<Vec<String>>()
                .join(", ");
            let files_label = if targets.len() > 3 {
                format!("Files ({} total): {}, ...", targets.len(), first_targets)
            } else {
                format!("Files: {}", first_targets)
            };

            let text = format!(
                "\n {}\n\n Template command (use %f for file name):\n > {}\n\n [Enter] Execute   [Esc] Cancel",
                files_label,
                input
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::DescribeFilePrompt { path, current_desc, input } => {
            let area = centered_rect(60, 30, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Describe File ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let text = format!(
                "\n File: {}\n Current Description: {}\n\n New Description:\n > {}\n\n [Enter] Save   [Esc] Cancel",
                file_name,
                current_desc,
                input
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::SelectGroupPrompt { mode, query } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let title = match mode {
                SelectMode::Add => " Select Group ",
                SelectMode::Remove => " Unselect Group ",
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title)
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let prompt_label = match mode {
                SelectMode::Add => "Select matching files:",
                SelectMode::Remove => "Unselect matching files:",
            };

            let text = format!(
                "\n {}\n\n > {}\n\n [Enter] Confirm   [Esc] Cancel",
                prompt_label,
                query
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::CreateLinkPrompt { src, dest_input, kind } => {
            let area = centered_rect(60, 30, size);
            f.render_widget(Clear, area);

            let title = match kind {
                LinkKind::Symbolic => " Create Symbolic Link ",
                LinkKind::Hard => " Create Hard Link ",
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(title)
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let src_name = src.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let text = format!(
                "\n Source: {}\n Link Path Destination:\n\n > {}\n\n [Enter] Confirm   [Esc] Cancel",
                src_name,
                dest_input
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::FilePanelFilterPrompt { input } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" File Mask Filter ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = format!(
                "\n Enter mask filter (e.g. *.rs; empty to show all):\n\n > {}\n\n [Enter] Apply   [Esc] Cancel",
                input
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::WipeConfirm { paths } => {
            let area = centered_rect(55, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(" WARNING: Secure Wipe Confirm ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = format!(
                "\n Are you sure you want to SECURELY WIPE {} item(s)?\n This writes over files and is IRRECOVERABLE.\n\n [Enter] Wipe   [Esc] Cancel",
                paths.len()
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(Color::LightRed));

            f.render_widget(paragraph, area);
        }
        PopupType::SaveSetupConfirm => {
            let area = centered_rect(45, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(" Save Setup ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = "\n Save configuration and current layout settings?\n\n [Enter] Confirm   [Esc] Cancel";
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::SortModesDialog { current, reverse, cursor_idx } => {
            let area = centered_rect(45, 35, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(" Sort Modes ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let fields = [
                SortField::Name,
                SortField::Extension,
                SortField::Size,
                SortField::Date,
                SortField::Unsorted,
            ];

            let mut lines = Vec::new();
            for (i, field) in fields.iter().enumerate() {
                let is_cursor = i == *cursor_idx;
                let is_selected = field == current;
                let active_marker = if is_selected { "√" } else { " " };
                let cursor_marker = if is_cursor { ">" } else { " " };

                let name = match field {
                    SortField::Name => "Name",
                    SortField::Extension => "Extension",
                    SortField::Size => "Size",
                    SortField::Date => "Date",
                    SortField::Unsorted => "Unsorted",
                };

                let line_str = format!(" {} [{}] {} ", cursor_marker, active_marker, name);
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

            // Reverse setting row
            let is_reverse_cursor = *cursor_idx == fields.len();
            let reverse_marker = if *reverse { "√" } else { " " };
            let cursor_marker = if is_reverse_cursor { ">" } else { " " };
            let line_str = format!(" {} [{}] Reverse order ", cursor_marker, reverse_marker);
            let style = if is_reverse_cursor {
                Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.selection_fg))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(parse_color(&theme.popup_fg))
            };
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(line_str, style)));

            let paragraph = Paragraph::new(lines).block(block);
            f.render_widget(paragraph, area);
        }
        PopupType::QuickViewPanel { .. } => {}
        PopupType::FileAttributesDialog { attrs, mode_input } => {
            let area = centered_rect(65, 45, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(" File Attributes ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let path_str = attrs.path.to_string_lossy();
            let file_name = attrs.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path_str.to_string());
            let readonly_status = if attrs.readonly { "Yes" } else { "No" };

            let format_time = |t: Option<std::time::SystemTime>| {
                t.map(|st| {
                    let datetime: chrono::DateTime<chrono::Local> = st.into();
                    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_else(|| "N/A".to_string())
            };

            let modified_str = format_time(attrs.modified);
            let created_str = format_time(attrs.created);

            let text = format!(
                "\n Name: {}\n Path: {}\n Size: {} bytes\n Owner: {}\n Links: {}\n Readonly: {}\n Modified: {}\n Created: {}\n\n Unix Permissions (octal):\n > {}\n\n [Enter] Save   [Esc] Cancel",
                file_name,
                path_str,
                attrs.size,
                attrs.owner,
                attrs.nlinks,
                readonly_status,
                modified_str,
                created_str,
                mode_input
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::CommandHistoryList { entries, cursor_idx } => {
            let area = centered_rect(60, 50, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Command History ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if entries.is_empty() {
                let paragraph = Paragraph::new("\n No command history.\n\n [Esc] Close")
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let list_height = inner.height.saturating_sub(2) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                for (i, entry) in entries.iter().enumerate().skip(scroll_start).take(list_height) {
                    let is_cursor = i == *cursor_idx;
                    let style = if is_cursor {
                        Style::default()
                            .bg(parse_color(&theme.selection_bg))
                            .fg(parse_color(&theme.selection_fg))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(parse_color(&theme.popup_fg))
                    };
                    lines.push(Line::from(Span::styled(format!(" {} ", entry), style)));
                }

                let hint = Line::from(Span::styled(
                    " [Enter] Execute command  [Esc] Close ",
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph = Paragraph::new(lines)
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
        }
        PopupType::FileViewHistoryList { entries, cursor_idx } => {
            let area = centered_rect(65, 50, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" File View History ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if entries.is_empty() {
                let paragraph = Paragraph::new("\n No viewed file history.\n\n [Esc] Close")
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let list_height = inner.height.saturating_sub(2) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                for (i, entry) in entries.iter().enumerate().skip(scroll_start).take(list_height) {
                    let is_cursor = i == *cursor_idx;
                    let display = entry.to_string_lossy();
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
                    " [Enter] View / Edit File  [Esc] Close ",
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph = Paragraph::new(lines)
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
        }
        PopupType::FoldersHistoryList { entries, cursor_idx } => {
            let area = centered_rect(65, 50, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Folder Navigation History ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if entries.is_empty() {
                let paragraph = Paragraph::new("\n No folder history.\n\n [Esc] Close")
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let list_height = inner.height.saturating_sub(2) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                for (i, entry) in entries.iter().enumerate().skip(scroll_start).take(list_height) {
                    let is_cursor = i == *cursor_idx;
                    let display = entry.to_string_lossy();
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
                    " [Enter] Jump to Folder  [Esc] Close ",
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph = Paragraph::new(lines)
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
        }
        PopupType::CompareFoldersResult { diff, cursor_idx } => {
            let area = centered_rect(75, 60, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Folder Compare Results ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if diff.is_empty() {
                let paragraph = Paragraph::new("\n All files are identical.\n\n [Esc] Close")
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let list_height = inner.height.saturating_sub(2) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                for (i, entry) in diff.iter().enumerate().skip(scroll_start).take(list_height) {
                    let is_cursor = i == *cursor_idx;
                    let (status_text, color) = match entry.status {
                        CompareStatus::OnlyLeft => ("Only in Left", Color::LightGreen),
                        CompareStatus::OnlyRight => ("Only in Right", Color::LightYellow),
                        CompareStatus::Different => ("Different Size/Time", Color::LightRed),
                        CompareStatus::Equal => ("Equal", Color::DarkGray),
                    };

                    let line_str = format!(" {:<40} | {:<20} ", entry.name, status_text);
                    let style = if is_cursor {
                        Style::default()
                            .bg(parse_color(&theme.selection_bg))
                            .fg(parse_color(&theme.selection_fg))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(color)
                    };
                    lines.push(Line::from(Span::styled(line_str, style)));
                }

                let hint = Line::from(Span::styled(
                    " [Esc] Close  (Differences are automatically tagged in active panel) ",
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph = Paragraph::new(lines)
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
        }
        PopupType::TaskListDialog { tasks, cursor_idx } => {
            let area = centered_rect(70, 60, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Task List (OS Processes) ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if tasks.is_empty() {
                let paragraph = Paragraph::new("\n No processes listed.\n\n [Esc] Close")
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let list_height = inner.height.saturating_sub(2) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                // Table header
                lines.push(Line::from(vec![
                    Span::styled(format!(" {:<8} | {:<35} | {:<12} ", "PID", "Process Name", "Memory (MB)"), Style::default().add_modifier(Modifier::UNDERLINED)),
                ]));

                for (i, task) in tasks.iter().enumerate().skip(scroll_start).take(list_height) {
                    let is_cursor = i == *cursor_idx;
                    let mem_mb = (task.memory_kb as f64) / 1024.0;
                    let line_str = format!(" {:<8} | {:<35} | {:<12.1} ", task.pid, task.name, mem_mb);
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

                let hint = Line::from(Span::styled(
                    " [Del / Alt+Del] Kill process  [Esc] Close ",
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph = Paragraph::new(lines)
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
        }
        PopupType::FileAssociationsDialog { rules, cursor_idx } => {
            let area = centered_rect(75, 60, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" File Associations ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if rules.is_empty() {
                let paragraph = Paragraph::new("\n No rules configured.\n\n [Esc] Close")
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let list_height = inner.height.saturating_sub(2) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                lines.push(Line::from(vec![
                    Span::styled(format!(" {:<15} | {:<30} | {:<30} ", "Mask", "Open Command", "View Command (F3)"), Style::default().add_modifier(Modifier::UNDERLINED)),
                ]));

                for (i, rule) in rules.iter().enumerate().skip(scroll_start).take(list_height) {
                    let is_cursor = i == *cursor_idx;
                    let view_cmd_str = rule.view_cmd.as_deref().unwrap_or("(Same as open)");
                    let line_str = format!(" {:<15} | {:<30} | {:<30} ", rule.mask, rule.open_cmd, view_cmd_str);
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

                let hint = Line::from(Span::styled(
                    " [Esc] Close ",
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph = Paragraph::new(lines)
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
        }
        PopupType::ArchiveCommandsMenu { archive_path, items, cursor_idx } => {
            let area = centered_rect(60, 45, size);
            f.render_widget(Clear, area);

            let title = format!(" Archive Commands: {} ", archive_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(title)
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if items.is_empty() {
                let paragraph = Paragraph::new("\n No archive commands available.\n\n [Esc] Close")
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let list_height = inner.height.saturating_sub(2) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                for (i, item) in items.iter().enumerate().skip(scroll_start).take(list_height) {
                    let is_cursor = i == *cursor_idx;
                    let line_str = if is_cursor {
                        format!(" >  {} ", item)
                    } else {
                        format!("    {} ", item)
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

                let hint = Line::from(Span::styled(
                    " [Enter] Execute Option  [Esc] Close ",
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
