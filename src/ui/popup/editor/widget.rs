use super::highlight::highlight_line;
use crate::app::editor::EditorState;
use crate::app::state::PopupType;
use crate::app::text_input;
use crate::config::localization::t;
use crate::ui::text_width::{display_width, expand_tabs, skip_columns};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
};
use unicode_segmentation::UnicodeSegmentation;

/// Width of the line-number gutter (`"1234 │ "`).
const GUTTER_WIDTH: u16 = 7;

/// Display settings of the editor screen.
#[derive(Debug, Clone, Copy)]
pub struct EditorView {
    pub tab_size: usize,
    pub show_line_numbers: bool,
}

pub fn render_editor_widget(
    f: &mut Frame,
    area: Rect,
    ed: &EditorState,
    view: EditorView,
    theme: &crate::config::theme::Theme,
    active_popup: Option<&PopupType>,
) {
    let title = t("editor_title")
        .replacen("{}", &ed.path.to_string_lossy(), 1)
        .replacen("{}", if ed.is_dirty() { "*" } else { "" }, 1);

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

    // Check if there is an active search query from the search popup
    let search_info = match active_popup {
        Some(PopupType::EditorSearchPrompt {
            query,
            case_sensitive,
            ..
        }) if !query.is_empty() => Some((query.as_str(), *case_sensitive)),
        _ => None,
    };
    let normal_style = Style::default().fg(parse_color(&theme.panel_fg));
    let highlight_style = Style::default()
        .bg(parse_color(&theme.selection_bg))
        .fg(parse_color(&theme.marked_fg))
        .add_modifier(ratatui::style::Modifier::BOLD);

    // Horizontal scroll: keep the cursor column inside the text area.
    let gutter = if view.show_line_numbers {
        GUTTER_WIDTH
    } else {
        0
    };
    let text_width = edit_area.width.saturating_sub(gutter).max(1) as usize;
    let current_line = ed.current_line();
    let (before_cursor, _, _) = text_input::split_at_cursor(current_line, ed.cursor_x);
    let cursor_col = display_width(&expand_tabs(before_cursor, view.tab_size));
    let scroll_x = (cursor_col + 1).saturating_sub(text_width);

    let text: Vec<ratatui::text::Line> = ed
        .lines
        .iter()
        .enumerate()
        .skip(ed.scroll_y)
        .take(edit_area.height as usize)
        .map(|(idx, line)| {
            let expanded = expand_tabs(line, view.tab_size);
            let line = skip_columns(&expanded, scroll_x).to_string();
            let mut spans = Vec::new();
            if view.show_line_numbers {
                spans.push(ratatui::text::Span::raw(format!("{:>4} │ ", idx + 1)));
            }
            match search_info {
                Some((q, cs)) => {
                    spans.extend(highlight_line(&line, q, cs, normal_style, highlight_style))
                }
                None => spans.push(ratatui::text::Span::raw(line)),
            }
            ratatui::text::Line::from(spans)
        })
        .collect();

    f.render_widget(block, area);
    f.render_widget(Paragraph::new(text).style(normal_style), edit_area);

    let mut flags = ed.format.line_ending.label().to_string();
    if ed.stamp.read_only {
        flags.push_str(" | ");
        flags.push_str(&t("editor_read_only_flag"));
    }
    let status_text = t("editor_status_text")
        .replacen("{}", &current_line.graphemes(true).count().to_string(), 1)
        .replacen("{}", &ed.lines.len().to_string(), 1)
        .replacen("{}", &(ed.cursor_y + 1).to_string(), 1)
        .replacen(
            "{}",
            &(text_input::grapheme_col(current_line, ed.cursor_x) + 1).to_string(),
            1,
        )
        .replacen("{}", &flags, 1);
    let status_para = Paragraph::new(status_text).style(
        Style::default()
            .bg(parse_color(&theme.header_fg))
            .fg(parse_color(&theme.header_bg)),
    );
    f.render_widget(status_para, status_area);

    // Draw the terminal blinking cursor at the editing position
    let cursor_x = edit_area.x + gutter + (cursor_col - scroll_x) as u16;
    let cursor_y = edit_area.y + ed.cursor_y.saturating_sub(ed.scroll_y) as u16;

    if cursor_x < edit_area.x + edit_area.width && cursor_y < edit_area.y + edit_area.height {
        f.set_cursor_position((cursor_x, cursor_y));
    }
}
