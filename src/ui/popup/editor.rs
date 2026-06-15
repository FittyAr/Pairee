use super::centered_rect;
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::path::Path;

fn highlight_line(
    line: &str,
    query: &str,
    case_sensitive: bool,
    normal_style: Style,
    highlight_style: Style,
) -> Vec<ratatui::text::Span<'static>> {
    use ratatui::text::Span;
    if query.is_empty() {
        return vec![Span::styled(line.to_string(), normal_style)];
    }

    let mut spans = Vec::new();
    let query_len = query.chars().count();
    let line_chars: Vec<char> = line.chars().collect();
    let line_len = line_chars.len();

    let query_lower: Vec<char> = if case_sensitive {
        query.chars().collect()
    } else {
        query.to_lowercase().chars().collect()
    };

    let line_lower: Vec<char> = if case_sensitive {
        line_chars.clone()
    } else {
        line.to_lowercase().chars().collect()
    };

    let mut i = 0;
    while i < line_len {
        let mut matches = false;
        if i + query_len <= line_lower.len() {
            matches = true;
            for j in 0..query_len {
                if line_lower[i + j] != query_lower[j] {
                    matches = false;
                    break;
                }
            }
        }

        if matches {
            let match_str: String = line_chars[i..i + query_len].iter().collect();
            spans.push(Span::styled(match_str, highlight_style));
            i += query_len;
        } else {
            let mut normal_str = String::new();
            normal_str.push(line_chars[i]);
            i += 1;

            while i < line_len {
                let mut sub_matches = false;
                if i + query_len <= line_lower.len() {
                    sub_matches = true;
                    for j in 0..query_len {
                        if line_lower[i + j] != query_lower[j] {
                            sub_matches = false;
                            break;
                        }
                    }
                }
                if sub_matches {
                    break;
                }
                normal_str.push(line_chars[i]);
                i += 1;
            }
            spans.push(Span::styled(normal_str, normal_style));
        }
    }

    spans
}

pub fn render_editor_widget(
    f: &mut Frame,
    area: Rect,
    path: &Path,
    lines: &[String],
    cursor_x: usize,
    cursor_y: usize,
    scroll_y: usize,
    is_dirty: bool,
    theme: &crate::config::theme::Theme,
    active_popup: &Option<PopupType>,
) {
    let title = t("editor_title")
        .replacen("{}", &path.to_string_lossy(), 1)
        .replacen("{}", if is_dirty { "*" } else { "" }, 1);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(parse_color(&theme.panel_border)))
        .title(ratatui::text::Span::styled(
            title,
            Style::default()
                .fg(parse_color(&theme.header_fg))
                .add_modifier(ratatui::style::Modifier::BOLD),
        ))
        .style(Style::default().bg(parse_color(&theme.panel_bg)));

    let inner = block.inner(area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let edit_area = chunks[0];
    let status_area = chunks[1];

    let height = edit_area.height as usize;
    let visible_lines: Vec<String> = lines.iter().skip(scroll_y).take(height).cloned().collect();

    // Check if there is an active search query from the search popup
    let search_info = match active_popup {
        Some(PopupType::EditorSearchPrompt {
            query,
            case_sensitive,
            ..
        }) if !query.is_empty() => Some((query.as_str(), *case_sensitive)),
        _ => None,
    };

    let mut text = Vec::new();
    for (idx, line) in visible_lines.into_iter().enumerate() {
        let line_num = scroll_y + idx + 1;
        let prefix = format!("{:>4} │ ", line_num);
        let mut spans = vec![ratatui::text::Span::raw(prefix)];

        if let Some((q, cs)) = search_info {
            let normal_style = Style::default().fg(parse_color(&theme.panel_fg));
            let highlight_style = Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.marked_fg))
                .add_modifier(ratatui::style::Modifier::BOLD);
            spans.extend(highlight_line(&line, q, cs, normal_style, highlight_style));
        } else {
            spans.push(ratatui::text::Span::raw(line));
        }
        text.push(ratatui::text::Line::from(spans));
    }

    let paragraph = Paragraph::new(text).style(Style::default().fg(parse_color(&theme.panel_fg)));

    f.render_widget(block, area);
    f.render_widget(paragraph, edit_area);

    let current_line_len = lines.get(cursor_y).map(|l| l.len()).unwrap_or(0);
    let status_text = t("editor_status_text")
        .replacen("{}", &current_line_len.to_string(), 1)
        .replacen("{}", &lines.len().to_string(), 1)
        .replacen("{}", &(cursor_y + 1).to_string(), 1)
        .replacen("{}", &(cursor_x + 1).to_string(), 1);
    let status_para = Paragraph::new(status_text).style(
        Style::default()
            .bg(parse_color(&theme.header_fg))
            .fg(parse_color(&theme.header_bg)),
    );
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

pub fn render_editor_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::EditorSearchPrompt {
            query,
            case_sensitive,
            cursor_idx,
        } => {
            use ratatui::layout::{Constraint, Direction, Layout};

            let search_area = centered_rect_fixed(50, 9, size);
            f.render_widget(Clear, search_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(t("editor_search_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(search_area);
            f.render_widget(block, search_area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // 0: Search query prompt + input line
                    Constraint::Length(1), // 1: Separator line
                    Constraint::Length(1), // 2: Case sensitive checkbox
                    Constraint::Min(1),    // 3: Spacer
                    Constraint::Length(1), // 4: Buttons
                ])
                .split(inner);

            let act_style = Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg));
            let norm_style = Style::default().fg(parse_color(&theme.popup_fg));

            // Search query text
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
                    t("search_query_label"),
                    q_text
                ))
                .style(q_style),
                chunks[0],
            );

            // Separator
            let sep_style = Style::default().fg(parse_color(&theme.popup_border));
            let sep_str = ratatui::symbols::line::HORIZONTAL.repeat(inner.width as usize);
            f.render_widget(Paragraph::new(sep_str).style(sep_style), chunks[1]);

            // Case sensitive checkbox
            let cs_pref = if *cursor_idx == 1 { "► " } else { "  " };
            let cs_style = if *cursor_idx == 1 {
                act_style
            } else {
                norm_style
            };
            let cs_chk = if *case_sensitive { "[x]" } else { "[ ]" };
            f.render_widget(
                Paragraph::new(format!("{}{} {}", cs_pref, cs_chk, t("sys_case_sensitive")))
                    .style(cs_style),
                chunks[2],
            );

            // Buttons
            let b1_style = if *cursor_idx == 2 {
                act_style
            } else {
                norm_style
            };
            let b2_style = if *cursor_idx == 3 {
                act_style
            } else {
                norm_style
            };
            let btns = ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(t("btn_search_bracket"), b1_style),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_cancel_bracket"), b2_style),
            ]);
            f.render_widget(
                Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
                chunks[4],
            );

            true
        }
        PopupType::ConfirmDiscardEditorChanges => {
            let area = centered_rect(50, 20, size);
            f.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(t("editor_discard_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));
            let text = ratatui::text::Line::from(vec![ratatui::text::Span::styled(
                t("editor_discard_prompt"),
                Style::default().fg(parse_color(&theme.popup_fg)),
            )]);
            let para = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(para, area);
            true
        }
        _ => false,
    }
}

fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
    use ratatui::layout::{Constraint, Direction, Layout};
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height.min(r.height)),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width.min(r.width)),
            Constraint::Min(0),
        ])
        .split(popup_layout[1])[1]
}
