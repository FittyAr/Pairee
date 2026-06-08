use crate::app::context::AppContext;
use crate::app::state::{
    ActivePanel, AppState, CompareStatus, LinkKind, PopupType, SelectMode, SortField,
};
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
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let edit_area = chunks[0];
    let status_area = chunks[1];

    let height = edit_area.height as usize;
    let visible_lines: Vec<String> = lines.iter().skip(scroll_y).take(height).cloned().collect();

    let mut text = Vec::new();
    for (idx, line) in visible_lines.into_iter().enumerate() {
        let line_num = scroll_y + idx + 1;
        let prefix = format!("{:>4} │ ", line_num);
        text.push(ratatui::text::Line::from(format!("{}{}", prefix, line)));
    }

    let paragraph = Paragraph::new(text).style(Style::default().fg(Color::White));

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
    let status_para =
        Paragraph::new(status_text).style(Style::default().bg(Color::Cyan).fg(Color::Black));
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
            render_editor_widget(
                f, area, path, lines, *cursor_x, *cursor_y, *scroll_y, *is_dirty, theme,
            );
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
            render_editor_widget(
                f, area, path, lines, *cursor_x, *cursor_y, *scroll_y, *is_dirty, theme,
            );

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
        PopupType::SearchPrompt {
            query,
            content_query,
            search_root,
            focus_content,
        } => {
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

                for (i, path) in results
                    .iter()
                    .enumerate()
                    .skip(scroll_start)
                    .take(list_height)
                {
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

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
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

            for (i, node) in nodes
                .iter()
                .enumerate()
                .skip(scroll_start)
                .take(list_height)
            {
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

            let paragraph =
                Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
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
                .map(|p| {
                    p.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                })
                .collect::<Vec<String>>()
                .join(", ");
            let files_label = if targets.len() > 3 {
                format!("Files ({} total): {}, ...", targets.len(), first_targets)
            } else {
                format!("Files: {}", first_targets)
            };

            let text = format!(
                "\n {}\n\n Template command (use %f for file name):\n > {}\n\n [Enter] Execute   [Esc] Cancel",
                files_label, input
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::DescribeFilePrompt {
            path,
            current_desc,
            input,
        } => {
            let area = centered_rect(60, 30, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Describe File ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let text = format!(
                "\n File: {}\n Current Description: {}\n\n New Description:\n > {}\n\n [Enter] Save   [Esc] Cancel",
                file_name, current_desc, input
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
                prompt_label, query
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, area);
        }
        PopupType::CreateLinkPrompt {
            src,
            dest_input,
            kind,
        } => {
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

            let src_name = src
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let text = format!(
                "\n Source: {}\n Link Path Destination:\n\n > {}\n\n [Enter] Confirm   [Esc] Cancel",
                src_name, dest_input
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
        PopupType::SortModesDialog {
            current,
            reverse,
            cursor_idx,
        } => {
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
            let file_name = attrs
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path_str.to_string());
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
        PopupType::CommandHistoryList {
            entries,
            cursor_idx,
        } => {
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

                for (i, entry) in entries
                    .iter()
                    .enumerate()
                    .skip(scroll_start)
                    .take(list_height)
                {
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

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
        }
        PopupType::FileViewHistoryList {
            entries,
            cursor_idx,
        } => {
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

                for (i, entry) in entries
                    .iter()
                    .enumerate()
                    .skip(scroll_start)
                    .take(list_height)
                {
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

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
        }
        PopupType::FoldersHistoryList {
            entries,
            cursor_idx,
        } => {
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

                for (i, entry) in entries
                    .iter()
                    .enumerate()
                    .skip(scroll_start)
                    .take(list_height)
                {
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

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
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

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
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
                lines.push(Line::from(vec![Span::styled(
                    format!(
                        " {:<8} | {:<35} | {:<12} ",
                        "PID", "Process Name", "Memory (MB)"
                    ),
                    Style::default().add_modifier(Modifier::UNDERLINED),
                )]));

                for (i, task) in tasks
                    .iter()
                    .enumerate()
                    .skip(scroll_start)
                    .take(list_height)
                {
                    let is_cursor = i == *cursor_idx;
                    let mem_mb = (task.memory_kb as f64) / 1024.0;
                    let line_str =
                        format!(" {:<8} | {:<35} | {:<12.1} ", task.pid, task.name, mem_mb);
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

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
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

                lines.push(Line::from(vec![Span::styled(
                    format!(
                        " {:<15} | {:<30} | {:<30} ",
                        "Mask", "Open Command", "View Command (F3)"
                    ),
                    Style::default().add_modifier(Modifier::UNDERLINED),
                )]));

                for (i, rule) in rules
                    .iter()
                    .enumerate()
                    .skip(scroll_start)
                    .take(list_height)
                {
                    let is_cursor = i == *cursor_idx;
                    let view_cmd_str = rule.view_cmd.as_deref().unwrap_or("(Same as open)");
                    let line_str = format!(
                        " {:<15} | {:<30} | {:<30} ",
                        rule.mask, rule.open_cmd, view_cmd_str
                    );
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

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
        }
        PopupType::ArchiveCommandsMenu {
            archive_path,
            items,
            cursor_idx,
        } => {
            let area = centered_rect(60, 45, size);
            f.render_widget(Clear, area);

            let title = format!(
                " Archive Commands: {} ",
                archive_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            );
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

                for (i, item) in items
                    .iter()
                    .enumerate()
                    .skip(scroll_start)
                    .take(list_height)
                {
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

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
        }
        PopupType::ConfigurationDialog {
            active_tab,
            cursor_idx,
            editing_value,
            edit_buffer,
            settings,
        } => {
            let area = centered_rect(85, 85, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(" Configuration Settings ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Tab headers
                    Constraint::Length(1), // Separator
                    Constraint::Min(1),    // Tab contents
                    Constraint::Length(1), // Bottom separator
                    Constraint::Length(1), // Hint/Status bar
                ])
                .split(inner);

            let header_area = chunks[0];
            let separator_area = chunks[1];
            let content_area = chunks[2];
            let bottom_sep_area = chunks[3];
            let hint_area = chunks[4];

            f.render_widget(
                Paragraph::new("─".repeat(inner.width as usize))
                    .style(Style::default().fg(Color::DarkGray)),
                separator_area,
            );
            f.render_widget(
                Paragraph::new("─".repeat(inner.width as usize))
                    .style(Style::default().fg(Color::DarkGray)),
                bottom_sep_area,
            );

            let tab_titles = [
                " System ",
                " Panel ",
                " Interface ",
                " Confirmations ",
                " Language & Plugins ",
                " Editor/Viewer ",
                " Colors ",
            ];
            let mut tab_spans = Vec::new();
            for (i, title) in tab_titles.iter().enumerate() {
                let is_active = i == *active_tab;
                let style = if is_active {
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(parse_color(&theme.popup_fg))
                };
                tab_spans.push(Span::styled(format!("  [{}]  ", title), style));
            }
            f.render_widget(Paragraph::new(Line::from(tab_spans)), header_area);

            let mut rows: Vec<(String, bool)> = Vec::new();

            match active_tab {
                0 => {
                    rows.push((
                        format!(
                            "[{}] Delete to Recycle Bin",
                            if settings.delete_to_recycle_bin {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Use system copy routine",
                            if settings.use_system_copy_routine {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Copy files opened for writing",
                            if settings.copy_files_opened_for_writing {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Scan symbolic links",
                            if settings.scan_symbolic_links {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Save commands history",
                            if settings.save_commands_history {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Save folders history",
                            if settings.save_folders_history {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Save view and edit history",
                            if settings.save_view_and_edit_history {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Use Windows registered types",
                            if settings.use_windows_registered_types {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Automatic update of environment variables",
                            if settings.automatic_update_env_variables {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push(("Request administrator rights:".to_string(), false));
                    rows.push((
                        format!(
                            "  [{}] For modification",
                            if settings.req_admin_modification {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "  [{}] For reading",
                            if settings.req_admin_reading { "x" } else { " " }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Use additional privileges",
                            if settings.req_admin_use_additional_privileges {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!("Sorting collation: < {} >", settings.sorting_collation),
                        false,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Treat digits as numbers",
                            if settings.treat_digits_as_numbers {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Case sensitive",
                            if settings.case_sensitive_sort {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Auto save setup",
                            if settings.auto_save_setup { "x" } else { " " }
                        ),
                        false,
                    ));
                }
                1 => {
                    rows.push((
                        format!(
                            "[{}] Show hidden and system files",
                            if settings.show_hidden { "x" } else { " " }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Highlight files",
                            if settings.highlight_files { "x" } else { " " }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Select folders",
                            if settings.select_folders { "x" } else { " " }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Right click selects files",
                            if settings.right_click_selects_files {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Sort folder names by extension",
                            if settings.sort_folder_names_by_extension {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Allow reverse sort modes",
                            if settings.sort_reverse { "x" } else { " " }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "Disable automatic panel update if object count exceeds: [ {} ]",
                            settings.disable_panel_update_object_count
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Network drives autorefresh",
                            if settings.network_drives_autorefresh {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Show column titles",
                            if settings.show_column_titles {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Show status line",
                            if settings.show_status_line { "x" } else { " " }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Detect volume mount points",
                            if settings.detect_volume_mount_points {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Show files total information",
                            if settings.show_files_total_information {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Show free size",
                            if settings.show_free_size { "x" } else { " " }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Show scrollbar",
                            if settings.show_scrollbar { "x" } else { " " }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Show background screens number",
                            if settings.show_background_screens_number {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Show sort mode letter",
                            if settings.show_sort_mode_letter {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Show \"..\" in root folders",
                            if settings.show_dotdot_in_root_folders {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push(("InfoPanel settings:".to_string(), false));
                    rows.push((
                        format!(
                            "  [{}] Show power status",
                            if settings.infopanel_show_power_status {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Show CD drive parameters",
                            if settings.infopanel_show_cd_drive_parameters {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "  Computer name format: < {} >",
                            settings.infopanel_computer_name_format
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "  User name format: < {} >",
                            settings.infopanel_user_name_format
                        ),
                        false,
                    ));
                    rows.push((
                        "Groups of file masks: [Ins/Del/F4/F7/Ctrl+R]".to_string(),
                        false,
                    ));
                    rows.push((
                        "Edit panel modes: [Ins/Del/F4/Ctrl+Enter]".to_string(),
                        false,
                    ));
                    rows.push(("File descriptions:".to_string(), false));
                    rows.push((
                        format!("  Names: [ {} ]", settings.file_descriptions_list_names),
                        false,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Set \"hidden\" attribute to new lists",
                            if settings.file_descriptions_set_hidden {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Update read only description file",
                            if settings.file_descriptions_update_readonly {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "  Position of new descriptions: [ {} ]",
                            settings.file_descriptions_position
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "  Update mode: < {} >",
                            settings.file_descriptions_update_mode
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Use ANSI code page by default",
                            if settings.file_descriptions_use_ansi {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Save in UTF-8",
                            if settings.file_descriptions_save_utf8 {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "Folder description list names: [ {} ]",
                            settings.folder_description_list_names
                        ),
                        false,
                    ));
                }
                2 => {
                    rows.push((
                        format!(
                            "[{}] Clock",
                            if settings.interface_clock { "x" } else { " " }
                        ),
                        true,
                    ));
                    rows.push((
                        format!("[{}] Mouse", if settings.mouse_support { "x" } else { " " }),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Show key bar",
                            if settings.interface_show_key_bar {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Always show the menu bar",
                            if settings.interface_always_show_menu_bar {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "Screen saver: [ {} ] minutes",
                            settings.interface_screen_saver_minutes
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Show total copy progress indicator",
                            if settings.interface_show_total_copy_progress {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Show copying time information",
                            if settings.interface_show_copying_time {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Show total delete progress indicator",
                            if settings.interface_show_total_delete_progress {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Use Ctrl+PgUp to change drive",
                            if settings.interface_use_ctrl_pgup_change_drive {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Use Virtual Terminal for rendering",
                            if settings.interface_use_virtual_terminal {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Fullwidth-aware rendering",
                            if settings.interface_fullwidth_aware_rendering {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] ClearType-friendly redraw",
                            if settings.interface_cleartype_friendly_redraw {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!("Console icon: [ {} ]", settings.interface_console_icon),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Alternate for Administrator",
                            if settings.interface_console_icon_admin_alternate {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    if *editing_value && *cursor_idx == 14 {
                        rows.push((
                            format!("Far window title addons: [ {}◄ ]", edit_buffer),
                            false,
                        ));
                    } else {
                        rows.push((
                            format!(
                                "Far window title addons: [ {} ]",
                                settings.interface_window_title_addons
                            ),
                            true,
                        ));
                    }
                    rows.push(("Dialog settings:".to_string(), true));
                    rows.push((
                        format!(
                            "  [{}] History in dialog edit controls",
                            if settings.dialog_history_in_edit_controls {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Persistent blocks in edit controls",
                            if settings.dialog_persistent_blocks {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Del removes blocks in edit controls",
                            if settings.dialog_del_removes_blocks {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] AutoComplete in edit controls",
                            if settings.dialog_autocomplete {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Backspace deletes unchanged text",
                            if settings.dialog_backspace_deletes_unchanged {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Mouse click outside closes dialog",
                            if settings.dialog_mouse_click_outside_closes {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push(("Menu settings:".to_string(), true));
                    rows.push((
                        format!(
                            "  Left click outside: < {} >",
                            settings.menu_left_click_outside
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  Right click outside: < {} >",
                            settings.menu_right_click_outside
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  Middle click outside: < {} >",
                            settings.menu_middle_click_outside
                        ),
                        true,
                    ));
                    rows.push(("Command line settings:".to_string(), true));
                    rows.push((
                        format!(
                            "  [{}] Persistent blocks",
                            if settings.cmdline_persistent_blocks {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Del removes blocks",
                            if settings.cmdline_del_removes_blocks {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] AutoComplete",
                            if settings.cmdline_autocomplete {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  Set prompt format: [ {} ]",
                            settings.cmdline_prompt_format
                        ),
                        true,
                    ));
                    rows.push((
                        format!("  Use home dir: [ {} ]", settings.cmdline_use_home_dir),
                        true,
                    ));
                    rows.push(("AutoComplete settings:".to_string(), true));
                    rows.push((
                        format!(
                            "  [{}] Show a list",
                            if settings.autocomplete_show_list {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "    [{}] Modal mode",
                            if settings.autocomplete_modal_mode {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Append the first matched item",
                            if settings.autocomplete_append_first {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!("Keybindings preset: < {} >", settings.keybinding_preset),
                        false,
                    ));
                }
                3 => {
                    rows.push((
                        format!(
                            "[{}] Copy",
                            if settings.confirmations.confirm_copy {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Move",
                            if settings.confirmations.confirm_move {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Overwrite and delete R/O files",
                            if settings.confirmations.confirm_overwrite {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Drag and drop",
                            if settings.confirmations.confirm_drag_and_drop {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Delete",
                            if settings.confirmations.confirm_delete {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                    rows.push((
                        format!(
                            "[{}] Delete non-empty folders",
                            if settings.confirmations.confirm_delete_non_empty_folders {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Interrupt operation",
                            if settings.confirmations.confirm_interrupt_operation {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Disconnect network drive",
                            if settings.confirmations.confirm_disconnect_network_drive {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Delete SUBST-disk",
                            if settings.confirmations.confirm_delete_subst_disk {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Detach virtual disk",
                            if settings.confirmations.confirm_detach_virtual_disk {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] HotPlug-device removal",
                            if settings.confirmations.confirm_hotplug_removal {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Reload edited file",
                            if settings.confirmations.confirm_reload_edited_file {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Clear history list",
                            if settings.confirmations.confirm_clear_history_list {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Exit",
                            if settings.confirmations.confirm_quit {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        false,
                    ));
                }
                4 => {
                    rows.push((format!("Main language: < {} >", settings.language), false));
                    rows.push((
                        "Plugins configuration: [ArcLite | EMenu | HlfViewer | NetBox]".to_string(),
                        true,
                    ));
                    rows.push(("Plugins manager settings:".to_string(), true));
                    rows.push((
                        format!(
                            "  [{}] OEM plugins support",
                            if settings.plugins_manager_oem_support {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Scan symbolic links",
                            if settings.plugins_manager_scan_symlinks {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push(("  Plugin selection:".to_string(), true));
                    rows.push((
                        format!(
                            "    [{}] File processing",
                            if settings.plugins_manager_file_processing {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "      [{}] Show standard association",
                            if settings.plugins_manager_show_standard_association {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "        [{}] Even if only one plugin",
                            if settings.plugins_manager_even_if_one_found {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "    [{}] Search results (SetFindList)",
                            if settings.plugins_manager_search_results {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "    [{}] Prefix processing",
                            if settings.plugins_manager_prefix_processing {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                }
                5 => {
                    rows.push((
                        format!(
                            "[{}] Use external editor for F4 instead of Alt+F4",
                            if settings.editor_use_external {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    if *editing_value && *cursor_idx == 1 {
                        rows.push((format!("Editor command: [ {}◄ ]", edit_buffer), false));
                    } else {
                        rows.push((
                            format!("Editor command: [ {} ]", settings.default_editor),
                            false,
                        ));
                    }
                    rows.push(("Internal editor:".to_string(), true));
                    rows.push((
                        format!("  Expand tabs: < {} >", settings.editor_expand_tabs),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Persistent blocks",
                            if settings.editor_persistent_blocks {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Cursor beyond end of line",
                            if settings.editor_cursor_beyond_eol {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Del removes blocks",
                            if settings.editor_del_removes_blocks {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Select found",
                            if settings.editor_select_found {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Auto indent",
                            if settings.editor_auto_indent {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Cursor at the end",
                            if settings.editor_cursor_at_end {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!("  Tab size: [ {} ]", settings.editor_tab_size),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Show scrollbar",
                            if settings.editor_show_scrollbar {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Show white space",
                            if settings.editor_show_white_space {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Show line numbers",
                            if settings.editor_show_line_numbers {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Save file position",
                            if settings.editor_save_file_position {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Save bookmarks",
                            if settings.editor_save_bookmarks {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Allow editing files opened for writing",
                            if settings.editor_allow_editing_opened_writing {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Lock editing of read-only files",
                            if settings.editor_lock_editing_readonly {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Warn when opening read-only files",
                            if settings.editor_warn_opening_readonly {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Autodetect code page",
                            if settings.editor_autodetect_codepage {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  Default code page: < {} >",
                            settings.editor_default_codepage
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "[{}] Use external viewer for F3 instead of Alt+F3",
                            if settings.viewer_use_external {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    if *editing_value && *cursor_idx == 22 {
                        rows.push((format!("Viewer command: [ {}◄ ]", edit_buffer), false));
                    } else {
                        rows.push((
                            format!("Viewer command: [ {} ]", settings.viewer_command),
                            true,
                        ));
                    }
                    rows.push(("Internal viewer:".to_string(), true));
                    rows.push((
                        format!(
                            "  [{}] Persistent selection",
                            if settings.viewer_persistent_selection {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Show scrolling arrows",
                            if settings.viewer_show_scrolling_arrows {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!("  Tab size: [ {} ]", settings.viewer_tab_size),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Visible '\\0'",
                            if settings.viewer_visible_zero {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Show scrollbar",
                            if settings.viewer_show_scrollbar {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Save file position",
                            if settings.viewer_save_file_position {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Save view mode",
                            if settings.viewer_save_view_mode {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Save file code page",
                            if settings.viewer_save_file_codepage {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Save wrap mode",
                            if settings.viewer_save_wrap_mode {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Save bookmarks",
                            if settings.viewer_save_bookmarks {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Detect dump view mode",
                            if settings.viewer_detect_dump_view_mode {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  Maximum line width: [ {} ]",
                            settings.viewer_max_line_width
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  [{}] Autodetect code page",
                            if settings.viewer_autodetect_codepage {
                                "x"
                            } else {
                                " "
                            }
                        ),
                        true,
                    ));
                    rows.push((
                        format!(
                            "  Default code page: < {} >",
                            settings.viewer_default_codepage
                        ),
                        true,
                    ));
                    rows.push(("Code pages list: [Ctrl+H Ins Del F4]".to_string(), true));
                }
                6 => {
                    rows.push((format!("Theme: < {} >", settings.theme), false));
                    rows.push((
                        "Color groups: [ Panel | Dialog | Menu | clock | ... ]".to_string(),
                        true,
                    ));
                    rows.push((
                        "Files highlighting: [ +H | +S | +D | <exec> | <arc> | <temp> ]"
                            .to_string(),
                        true,
                    ));
                }
                _ => {}
            }

            rows.push(("[ OK ]".to_string(), false));
            rows.push(("[ Cancel ]".to_string(), false));

            let list_height = content_area.height as usize;
            let scroll_start = cursor_idx.saturating_sub(list_height / 2);
            let mut list_spans = Vec::new();

            for (i, (label, is_stub)) in
                rows.iter().enumerate().skip(scroll_start).take(list_height)
            {
                let is_cursor = i == *cursor_idx;

                let display_label = if *is_stub {
                    format!("{} *", label)
                } else {
                    label.clone()
                };

                let style = if is_cursor {
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD)
                } else if *is_stub {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(parse_color(&theme.popup_fg))
                };

                list_spans.push(Line::from(Span::styled(
                    format!("  {}  ", display_label),
                    style,
                )));
            }

            f.render_widget(Paragraph::new(list_spans), content_area);

            let hint_str = " * Unimplemented/Future feature  |  [Tab/Arrows] Navigate  [Space/Enter] Edit/Toggle  [F9] Save  [Esc] Cancel";
            let hint_widget = Paragraph::new(hint_str).style(Style::default().fg(Color::Yellow));
            f.render_widget(hint_widget, hint_area);
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
