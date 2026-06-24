use super::centered_rect;
use crate::app::state::{CompareStatus, PopupType};
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render_history_lists_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::CommandHistoryList {
            entries,
            cursor_idx,
        } => {
            let area = centered_rect(60, 50, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("history_command_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if entries.is_empty() {
                let paragraph = Paragraph::new(t("history_command_empty"))
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
                    t("history_command_hint"),
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
            true
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
                .title(t("history_view_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if entries.is_empty() {
                let paragraph = Paragraph::new(t("history_view_empty"))
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
                    t("history_view_hint"),
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
            true
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
                .title(t("history_folder_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if entries.is_empty() {
                let paragraph = Paragraph::new(t("history_folder_empty"))
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
                    t("history_folder_hint"),
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
            true
        }
        PopupType::SearchPrompt {
            query,
            content_query,
            search_root,
            case_sensitive,
            search_target,
            cursor_idx,
        } => {
            use ratatui::layout::{Constraint, Direction, Layout};

            let area = centered_rect(65, 40, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("prompt_search_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // 0: Search in root folder
                    Constraint::Length(2), // 1: File name pattern
                    Constraint::Length(2), // 2: Content query
                    Constraint::Length(1), // 3: Separator line
                    Constraint::Length(1), // 4: Case sensitive
                    Constraint::Length(1), // 5: Search target
                    Constraint::Min(1),    // 6: Spacer
                    Constraint::Length(1), // 7: Buttons
                ])
                .split(inner);

            let act_style = Style::default().bg(Color::Cyan).fg(Color::Black);
            let norm_style = Style::default().fg(parse_color(&theme.popup_fg));

            // Search root path
            let root_str = search_root.to_string_lossy();
            f.render_widget(
                Paragraph::new(format!(" {}: {}", t("prompt_find_folder"), root_str))
                    .style(norm_style),
                chunks[0],
            );

            // File name pattern
            let q_pref = if *cursor_idx == 0 { "► " } else { "  " };
            let q_style = if *cursor_idx == 0 {
                act_style
            } else {
                norm_style
            };
            let q_text = if *cursor_idx == 0 {
                format!("{}_", query)
            } else {
                query.clone()
            };
            f.render_widget(
                Paragraph::new(format!(
                    "{}{}\n   > {}",
                    q_pref,
                    t("prompt_find_pattern"),
                    q_text
                ))
                .style(q_style),
                chunks[1],
            );

            // Content query
            let c_pref = if *cursor_idx == 1 { "► " } else { "  " };
            let c_style = if *cursor_idx == 1 {
                act_style
            } else {
                norm_style
            };
            let c_text = if *cursor_idx == 1 {
                format!("{}_", content_query)
            } else {
                content_query.clone()
            };
            f.render_widget(
                Paragraph::new(format!(
                    "{}{}\n   > {}",
                    c_pref,
                    t("prompt_find_content"),
                    c_text
                ))
                .style(c_style),
                chunks[2],
            );

            // Separator line
            let sep_style = Style::default().fg(Color::Cyan);
            let sep_str = ratatui::symbols::line::HORIZONTAL.repeat(inner.width as usize);
            f.render_widget(Paragraph::new(sep_str).style(sep_style), chunks[3]);

            // Case sensitive checkbox
            let cs_pref = if *cursor_idx == 2 { "► " } else { "  " };
            let cs_style = if *cursor_idx == 2 {
                act_style
            } else {
                norm_style
            };
            let cs_chk = if *case_sensitive { "[x]" } else { "[ ]" };
            f.render_widget(
                Paragraph::new(format!("{}{} {}", cs_pref, cs_chk, t("sys_case_sensitive")))
                    .style(cs_style),
                chunks[4],
            );

            // Search target selection
            let target_pref = if *cursor_idx == 3 { "► " } else { "  " };
            let target_style = if *cursor_idx == 3 {
                act_style
            } else {
                norm_style
            };
            let target_val = match search_target {
                crate::fs::search::SearchTarget::Any => t("search_target_any"),
                crate::fs::search::SearchTarget::File => t("search_target_file"),
                crate::fs::search::SearchTarget::Directory => t("search_target_dir"),
            };
            f.render_widget(
                Paragraph::new(format!(
                    "{}{} < {} >",
                    target_pref,
                    t("search_target_label"),
                    target_val
                ))
                .style(target_style),
                chunks[5],
            );

            // Buttons
            let b1 = if *cursor_idx == 4 {
                act_style
            } else {
                norm_style
            };
            let b2 = if *cursor_idx == 5 {
                act_style
            } else {
                norm_style
            };
            let btns = ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(t("btn_ok_bracket"), b1),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_cancel_bracket"), b2),
            ]);
            f.render_widget(
                Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
                chunks[7],
            );

            true
        }
        PopupType::SearchResults {
            query,
            results,
            cursor_idx,
            searching,
        } => {
            let area = centered_rect(70, 60, size);
            f.render_widget(Clear, area);

            let mut title = t("search_results_title").replacen("{}", query, 1).replacen(
                "{}",
                &results.len().to_string(),
                1,
            );
            if *searching {
                title.push_str(&t("searching_suffix"));
            }
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title)
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if results.is_empty() {
                let text = if *searching {
                    t("searching_placeholder")
                } else {
                    t("search_results_empty")
                };
                let paragraph =
                    Paragraph::new(text).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let list_height = inner.height.saturating_sub(2) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                for (i, (path, is_dir)) in results
                    .iter()
                    .enumerate()
                    .skip(scroll_start)
                    .take(list_height)
                {
                    let is_cursor = i == *cursor_idx;
                    let display = path.to_string_lossy().to_string();
                    let prefix = if *is_dir { "📁 " } else { "📄 " };
                    let display_str = format!("{} {}", prefix, display);

                    let style = if is_cursor {
                        Style::default()
                            .bg(parse_color(&theme.selection_bg))
                            .fg(parse_color(&theme.selection_fg))
                            .add_modifier(Modifier::BOLD)
                    } else if *is_dir {
                        Style::default().fg(Color::LightBlue)
                    } else {
                        Style::default().fg(parse_color(&theme.popup_fg))
                    };
                    lines.push(Line::from(Span::styled(
                        format!(" {} ", display_str),
                        style,
                    )));
                }

                let hint = Line::from(Span::styled(
                    t("search_results_hint"),
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
            true
        }
        PopupType::TreeView {
            nodes,
            cursor_idx,
            caller: _,
        } => {
            let area = centered_rect(55, 70, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(t("tree_view_title"))
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
                t("tree_view_hint"),
                Style::default().fg(Color::DarkGray),
            ));
            lines.push(Line::from(""));
            lines.push(hint);

            let paragraph =
                Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
            f.render_widget(paragraph, inner);
            true
        }
        PopupType::CompareFoldersResult { diff, cursor_idx } => {
            let area = centered_rect(75, 60, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("compare_results_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if diff.is_empty() {
                let paragraph = Paragraph::new(t("compare_results_empty"))
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let list_height = inner.height.saturating_sub(2) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                for (i, entry) in diff.iter().enumerate().skip(scroll_start).take(list_height) {
                    let is_cursor = i == *cursor_idx;
                    let (status_text, color) = match entry.status {
                        CompareStatus::OnlyLeft => {
                            (t("compare_status_only_left"), Color::LightGreen)
                        }
                        CompareStatus::OnlyRight => {
                            (t("compare_status_only_right"), Color::LightYellow)
                        }
                        CompareStatus::Different => {
                            (t("compare_status_different"), Color::LightRed)
                        }
                        CompareStatus::Equal => (t("compare_status_equal"), Color::DarkGray),
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
                    t("compare_results_hint"),
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
            true
        }
        PopupType::TaskListDialog { tasks, cursor_idx } => {
            let area = centered_rect(70, 60, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("task_list_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if tasks.is_empty() {
                let paragraph = Paragraph::new(t("task_list_empty"))
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let list_height = inner.height.saturating_sub(4) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                // Table header
                lines.push(Line::from(vec![Span::styled(
                    format!(
                        " {:<8} | {:<35} | {:<12} ",
                        t("col_pid"),
                        t("col_process_name"),
                        t("col_memory")
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
                    t("task_list_hint"),
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
            true
        }
        PopupType::FileAssociationsDialog { rules, cursor_idx } => {
            let area = centered_rect(75, 60, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("prompt_associations_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if rules.is_empty() {
                let paragraph = Paragraph::new(t("associations_empty"))
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let list_height = inner.height.saturating_sub(2) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                lines.push(Line::from(vec![Span::styled(
                    format!(
                        " {:<15} | {:<30} | {:<30} ",
                        t("col_mask"),
                        t("col_open_command"),
                        t("col_view_command")
                    ),
                    Style::default().add_modifier(Modifier::UNDERLINED),
                )]));

                let same_as_open = t("associations_same_as_open");
                for (i, rule) in rules
                    .iter()
                    .enumerate()
                    .skip(scroll_start)
                    .take(list_height)
                {
                    let is_cursor = i == *cursor_idx;
                    let view_cmd_str = rule.view_cmd.as_deref().unwrap_or(&same_as_open);
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
                    t("hint_esc_close"),
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
            true
        }
        _ => false,
    }
}
