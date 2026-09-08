use crate::app::state::types::PluginWidget;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Gauge, List, ListItem, Paragraph, Row, Table, Wrap},
};

pub fn render_plugin_widget(
    f: &mut Frame,
    area: Rect,
    widget: &PluginWidget,
    block: Block,
    theme: &crate::config::theme::Theme,
    scroll: usize,
    scrollbar: Option<&ScrollbarUiState>,
) {
    match widget {
        PluginWidget::Paragraph(text) => {
            let viewport = area.height.saturating_sub(2) as usize;
            let content_len = text.lines().count().max(1);
            let para = Paragraph::new(text.as_str())
                .block(block)
                .style(Style::default().fg(parse_color(&theme.panel_fg)))
                .wrap(Wrap { trim: false })
                .scroll((scroll as u16, 0));
            f.render_widget(para, area);
            scrollbar::render_vertical_inside_block(
                f,
                area,
                content_len,
                viewport,
                scroll,
                theme,
                ScrollbarSurface::Panel,
                scrollbar,
                ScrollTargetId::QuickView,
            );
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
            let viewport = area.height.saturating_sub(2) as usize;
            let list_items: Vec<ListItem> = items
                .iter()
                .skip(scroll)
                .map(|item| ListItem::new(item.as_str()))
                .collect();
            let list = List::new(list_items)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.panel_fg)));
            f.render_widget(list, area);
            scrollbar::render_vertical_inside_block(
                f,
                area,
                items.len(),
                viewport,
                scroll,
                theme,
                ScrollbarSurface::Panel,
                scrollbar,
                ScrollTargetId::QuickView,
            );
        }
        PluginWidget::Table { headers, rows } => {
            let header_row =
                Row::new(headers.iter().map(|h| {
                    Span::styled(h.clone(), Style::default().add_modifier(Modifier::BOLD))
                }));
            let table_rows: Vec<Row> = rows
                .iter()
                .skip(scroll)
                .map(|r| Row::new(r.iter().map(|c| Span::raw(c.clone()))))
                .collect();
            let widths =
                vec![Constraint::Percentage(100 / headers.len().max(1) as u16); headers.len()];
            let table = Table::new(table_rows, widths)
                .header(header_row)
                .block(block)
                .style(Style::default().fg(parse_color(&theme.panel_fg)));
            f.render_widget(table, area);
        }
        PluginWidget::Span { text, style } => {
            let mut s = Style::default();
            if !style.is_empty() {
                s = s.fg(parse_color(style));
            }
            let para = Paragraph::new(Span::styled(text.clone(), s)).block(block);
            f.render_widget(para, area);
        }
        PluginWidget::Line(spans) => {
            let ratatui_spans: Vec<Span> = spans
                .iter()
                .map(|w| match w {
                    PluginWidget::Span { text, style } => {
                        let mut s = Style::default();
                        if !style.is_empty() {
                            s = s.fg(parse_color(style));
                        }
                        Span::styled(text.clone(), s)
                    }
                    _ => Span::raw(""),
                })
                .collect();
            let para = Paragraph::new(Line::from(ratatui_spans)).block(block);
            f.render_widget(para, area);
        }
    }
}
