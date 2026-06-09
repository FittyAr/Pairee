use super::centered_rect;
use crate::app::state::PopupType;
use crate::ui::theme_apply::parse_color;
use crate::config::localization::t;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::path::Path;

fn render_editor_widget(
    f: &mut Frame,
    area: Rect,
    path: &Path,
    lines: &[String],
    cursor_x: usize,
    cursor_y: usize,
    scroll_y: usize,
    is_dirty: bool,
    _theme: &crate::config::theme::Theme,
) {
    let title = t("editor_title")
        .replacen("{}", &path.to_string_lossy(), 1)
        .replacen("{}", if is_dirty { "*" } else { "" }, 1);

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
    let status_text = t("editor_status_text")
        .replacen("{}", &current_line_len.to_string(), 1)
        .replacen("{}", &lines.len().to_string(), 1)
        .replacen("{}", &(cursor_y + 1).to_string(), 1)
        .replacen("{}", &(cursor_x + 1).to_string(), 1);
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

pub fn render_editor_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    match popup {
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
            true
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
                .title(t("editor_search_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let text = t("editor_search_text").replacen("{}", query, 1);

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.popup_fg)));

            f.render_widget(paragraph, search_area);
            true
        }
        PopupType::InternalViewer { viewer } => {
            let area = centered_rect(95, 90, size);
            f.render_widget(Clear, area);
            crate::ui::viewer::render_viewer(f, area, viewer, theme);
            true
        }
        _ => false,
    }
}
