use super::highlight::highlight_line;
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
};
use std::path::Path;

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
    active_popup: Option<&PopupType>,
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
        f.set_cursor_position((editor_cursor_x, editor_cursor_y));
    }
}
