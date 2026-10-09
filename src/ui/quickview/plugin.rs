use crate::app::state::types::PluginWidget;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::scrollbar::{ScrollTarget, ScrollView};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Gauge, List, ListItem, Paragraph, Row, Table, Wrap},
};

type Theme = crate::config::theme::Theme;

pub fn render_plugin_widget(
    f: &mut Frame,
    area: Rect,
    widget: &PluginWidget,
    block: Block,
    theme: &Theme,
    scroll: usize,
    scrollbar: Option<&ScrollbarUiState>,
) {
    let panel_style = Style::default().fg(parse_color(&theme.panel_fg));
    match widget {
        PluginWidget::Paragraph(text) => {
            let para = Paragraph::new(text.as_str())
                .block(block)
                .style(panel_style)
                .wrap(Wrap { trim: false })
                .scroll((scroll as u16, 0));
            f.render_widget(para, area);
            let content_len = text.lines().count().max(1);
            render_scrollbar(f, area, theme, (content_len, scroll), scrollbar);
        }
        PluginWidget::Gauge { ratio, label } => {
            let gauge = Gauge::default()
                .block(block)
                .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
                .ratio(ratio.clamp(0.0, 1.0))
                .label(label.as_str());
            f.render_widget(gauge, area);
        }
        PluginWidget::List(items) => {
            let list_items: Vec<ListItem> = items
                .iter()
                .skip(scroll)
                .map(|item| ListItem::new(item.as_str()))
                .collect();
            let list = List::new(list_items).block(block).style(panel_style);
            f.render_widget(list, area);
            render_scrollbar(f, area, theme, (items.len(), scroll), scrollbar);
        }
        PluginWidget::Table { headers, rows } => {
            let table = table_widget(headers, rows, scroll)
                .block(block)
                .style(panel_style);
            f.render_widget(table, area);
        }
        PluginWidget::Span { .. } => {
            let para = Paragraph::new(plugin_span(widget)).block(block);
            f.render_widget(para, area);
        }
        PluginWidget::Line(spans) => {
            let line = Line::from(spans.iter().map(plugin_span).collect::<Vec<_>>());
            f.render_widget(Paragraph::new(line).block(block), area);
        }
    }
}

/// Quick View scrollbar inside the frame; `content` is (length, offset).
fn render_scrollbar(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    (content_len, offset): (usize, usize),
    scrollbar: Option<&ScrollbarUiState>,
) {
    scrollbar::render_vertical_inside_block(
        f,
        area,
        ScrollView {
            content_len,
            viewport_len: area.height.saturating_sub(2) as usize,
            offset,
        },
        theme,
        ScrollTarget {
            surface: ScrollbarSurface::Panel,
            hits: scrollbar,
            id: ScrollTargetId::QuickView,
        },
    );
}

/// Bold header and the rows from `scroll` on, in equal-width columns.
fn table_widget<'a>(headers: &[String], rows: &[Vec<String>], scroll: usize) -> Table<'a> {
    let header_row = Row::new(
        headers
            .iter()
            .map(|h| Span::styled(h.clone(), Style::default().add_modifier(Modifier::BOLD))),
    );
    let table_rows: Vec<Row> = rows
        .iter()
        .skip(scroll)
        .map(|r| Row::new(r.iter().map(|c| Span::raw(c.clone()))))
        .collect();
    let widths = vec![Constraint::Percentage(100 / headers.len().max(1) as u16); headers.len()];
    Table::new(table_rows, widths).header(header_row)
}

/// A `Span` widget with its optional color; other widgets give an empty span.
fn plugin_span(widget: &PluginWidget) -> Span<'static> {
    match widget {
        PluginWidget::Span { text, style } => {
            let mut s = Style::default();
            if !style.is_empty() {
                s = s.fg(parse_color(style));
            }
            Span::styled(text.clone(), s)
        }
        _ => Span::raw(""),
    }
}
