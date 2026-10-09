mod markdown;

pub use crate::ui::wrap::wrap_lines;
pub use markdown::parse_markdown_to_lines;

use super::super::centered_rect;
use crate::app::state::PopupType;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::scrollbar::{ScrollTarget, ScrollView};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

type Theme = crate::config::theme::Theme;

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &Theme,
    size: Rect,
    scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    let PopupType::Help {
        mode,
        docs,
        plugin_docs,
        active_tab,
        cursor_idx,
        scroll_y,
        active_content,
    } = popup
    else {
        return false;
    };
    let area = centered_rect(90, 85, size); // Expand to 90% width, 85% height
    f.render_widget(Clear, area);

    // Split into Left (list) and Right (content viewer)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // 25% for document list
            Constraint::Percentage(75), // 75% for content
        ])
        .split(area);

    // 1. Render Left panel (document selection list)
    let current_docs = if *active_tab == 0 { docs } else { plugin_docs };
    let list = DocList {
        focused: *mode == 0,
        active_tab: *active_tab,
        docs: current_docs,
        cursor_idx: *cursor_idx,
    };
    render_doc_list(f, chunks[0], theme, &list);

    // 2. Render Right panel (content viewer)
    let doc_title = current_docs
        .get(*cursor_idx)
        .map(|(t, _)| t.as_str())
        .unwrap_or(" Documentation ");
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(focus_color(*mode == 1, theme)))
        .title(format!(" {} ", doc_title))
        .style(Style::default().bg(parse_color(&theme.popup_bg)));
    match active_content {
        Some(content) => render_content(
            f,
            chunks[1],
            theme,
            right_block,
            content,
            *scroll_y,
            scrollbar,
        ),
        None => {
            let empty_paragraph = Paragraph::new(" No document loaded ").block(right_block);
            f.render_widget(empty_paragraph, chunks[1]);
        }
    }

    // 3. Render help hint at the bottom
    let hint_area = Rect {
        x: area.x + 2,
        y: area.y + area.height - 2,
        width: area.width.saturating_sub(4),
        height: 1,
    };
    let hint_text = " [Tab] Switch Panels  [Up/Down/j/k] Navigate/Scroll  [Esc] Close ";
    f.render_widget(
        Paragraph::new(hint_text)
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
    true
}

/// Yellow border on the focused pane, the theme border otherwise.
fn focus_color(focused: bool, theme: &Theme) -> Color {
    if focused {
        Color::Yellow
    } else {
        parse_color(&theme.popup_border)
    }
}

/// What the left pane shows: the tab and its documents.
struct DocList<'a> {
    focused: bool,
    active_tab: usize,
    docs: &'a [(String, std::path::PathBuf)],
    cursor_idx: usize,
}

/// Core/Plugins tab bar above the document titles.
fn render_doc_list(f: &mut Frame, left_area: Rect, theme: &Theme, list: &DocList) {
    let border_style = Style::default().fg(focus_color(list.focused, theme));
    let bg_style = Style::default().bg(parse_color(&theme.popup_bg));
    // Split left panel into Tabs and Document list
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Border + Tab bar
            Constraint::Min(1),    // List of documents
        ])
        .split(left_area);

    let dim = Style::default().fg(Color::DarkGray);
    let tab_style = |idx: usize| {
        if list.active_tab == idx {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            dim
        }
    };
    let tabs_line = Line::from(vec![
        Span::styled(" [ ", dim),
        Span::styled("Core Help", tab_style(0)),
        Span::styled(" ]  [ ", dim),
        Span::styled("Plugins Help", tab_style(1)),
        Span::styled(" ]", dim),
    ]);
    let tab_block = Block::default()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_style(border_style)
        .style(bg_style);
    f.render_widget(Paragraph::new(tabs_line).block(tab_block), left_chunks[0]);

    let list_items: Vec<ListItem> = list
        .docs
        .iter()
        .enumerate()
        .map(|(i, (doc_title, _))| {
            let style = if i == list.cursor_idx {
                Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.selection_fg))
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(parse_color(&theme.popup_fg))
            };
            ListItem::new(Line::from(vec![Span::styled(
                format!("  {}  ", doc_title),
                style,
            )]))
        })
        .collect();
    let left_block = Block::default()
        .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
        .border_style(border_style)
        .style(bg_style);
    let doc_list = List::new(list_items).block(left_block).style(bg_style);
    f.render_widget(doc_list, left_chunks[1]);
}

/// The document as wrapped Markdown, with a scrollbar when it overflows.
fn render_content(
    f: &mut Frame,
    right_area: Rect,
    theme: &Theme,
    right_block: Block,
    content: &str,
    scroll_y: usize,
    scrollbar: Option<&ScrollbarUiState>,
) {
    let parsed_lines = parse_markdown_to_lines(content);
    let inner_width = (right_area.width.saturating_sub(4)) as usize;
    let wrapped_lines = wrap_lines(parsed_lines, inner_width);
    let total_lines = wrapped_lines.len();

    let paragraph = Paragraph::new(wrapped_lines)
        .block(right_block)
        .scroll((scroll_y as u16, 0))
        .style(Style::default().fg(parse_color(&theme.popup_fg)));
    f.render_widget(paragraph, right_area);

    // Fractional scrollbar when content overflows the viewer pane
    let inner_height = right_area.height.saturating_sub(2) as usize;
    scrollbar::render_vertical_inside_block(
        f,
        right_area,
        ScrollView {
            content_len: total_lines,
            viewport_len: inner_height,
            offset: scroll_y,
        },
        theme,
        ScrollTarget {
            surface: ScrollbarSurface::Popup,
            hits: scrollbar,
            id: ScrollTargetId::HelpContent,
        },
    );
}
