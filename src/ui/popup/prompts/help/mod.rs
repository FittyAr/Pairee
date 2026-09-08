mod markdown;

pub use markdown::{parse_markdown_to_lines, wrap_lines};

use super::super::centered_rect;
use crate::app::state::PopupType;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    if let PopupType::Help {
        mode,
        docs,
        plugin_docs,
        active_tab,
        cursor_idx,
        scroll_y,
        active_content,
    } = popup
    {
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
        let left_area = chunks[0];
        let right_area = chunks[1];

        // 1. Render Left panel (document selection list)
        let left_border_color = if *mode == 0 {
            Color::Yellow
        } else {
            parse_color(&theme.popup_border)
        };

        // Split left panel into Tabs and Document list
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Border + Tab bar
                Constraint::Min(1),    // List of documents
            ])
            .split(left_area);
        let tab_area = left_chunks[0];
        let list_area = left_chunks[1];

        let tab_title_core = "Core Help";
        let tab_title_plugins = "Plugins Help";

        let core_style = if *active_tab == 0 {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let plugins_style = if *active_tab == 1 {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let tabs_line = Line::from(vec![
            Span::styled(" [ ", Style::default().fg(Color::DarkGray)),
            Span::styled(tab_title_core, core_style),
            Span::styled(" ]  [ ", Style::default().fg(Color::DarkGray)),
            Span::styled(tab_title_plugins, plugins_style),
            Span::styled(" ]", Style::default().fg(Color::DarkGray)),
        ]);

        let tab_block = Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(left_border_color))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));
        f.render_widget(Paragraph::new(tabs_line).block(tab_block), tab_area);

        let left_block = Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(left_border_color))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let current_docs = if *active_tab == 0 { docs } else { plugin_docs };

        let mut list_items = Vec::new();
        for (i, (doc_title, _)) in current_docs.iter().enumerate() {
            let style = if i == *cursor_idx {
                Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.selection_fg))
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(parse_color(&theme.popup_fg))
            };
            list_items.push(ListItem::new(Line::from(vec![Span::styled(
                format!("  {}  ", doc_title),
                style,
            )])));
        }

        let list = List::new(list_items)
            .block(left_block)
            .style(Style::default().bg(parse_color(&theme.popup_bg)));
        f.render_widget(list, list_area);

        // 2. Render Right panel (content viewer)
        let doc_title = current_docs
            .get(*cursor_idx)
            .map(|(t, _)| t.as_str())
            .unwrap_or(" Documentation ");
        let right_border_color = if *mode == 1 {
            Color::Yellow
        } else {
            parse_color(&theme.popup_border)
        };
        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(right_border_color))
            .title(format!(" {} ", doc_title))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        if let Some(content) = active_content {
            let parsed_lines = parse_markdown_to_lines(content);
            let inner_width = (right_area.width.saturating_sub(4)) as usize;
            let wrapped_lines = wrap_lines(parsed_lines, inner_width);

            let paragraph = Paragraph::new(wrapped_lines.clone())
                .block(right_block)
                .scroll((*scroll_y as u16, 0))
                .style(Style::default().fg(parse_color(&theme.popup_fg)));
            f.render_widget(paragraph, right_area);

            // Fractional scrollbar when content overflows the viewer pane
            let total_lines = wrapped_lines.len();
            let inner_height = right_area.height.saturating_sub(2) as usize;
            scrollbar::render_vertical_inside_block(
                f,
                right_area,
                total_lines,
                inner_height,
                *scroll_y,
                theme,
                ScrollbarSurface::Popup,
                scrollbar,
                ScrollTargetId::HelpContent,
            );
        } else {
            let empty_paragraph = Paragraph::new(" No document loaded ").block(right_block);
            f.render_widget(empty_paragraph, right_area);
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
    } else {
        false
    }
}
