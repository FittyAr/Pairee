use super::wrap_text;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier as StyleModifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub fn render_search(
    f: &mut Frame,
    list_area: Rect,
    detail_area: Rect,
    cursor_idx: usize,
    registry: &[(String, String, String, String)],
    is_searching: bool,
    theme: &crate::config::theme::Theme,
    border_style: Style,
    bg_style: Style,
) {
    let text_style = Style::default().fg(parse_color(&theme.popup_fg));
    let bold_style = text_style.add_modifier(StyleModifier::BOLD);

    let mut list_items = Vec::new();
    if is_searching {
        list_items.push(ListItem::new(Line::from(vec![Span::styled(
            t("plugin_search_searching"),
            Style::default().fg(Color::Yellow),
        )])));
    } else if registry.is_empty() {
        list_items.push(ListItem::new(Line::from(vec![Span::styled(
            t("plugin_search_no_results"),
            Style::default().fg(Color::DarkGray),
        )])));
    } else {
        for (i, (name, version, _, author)) in registry.iter().enumerate() {
            let style = if i == cursor_idx {
                Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.selection_fg))
                    .add_modifier(StyleModifier::BOLD)
            } else {
                Style::default().fg(parse_color(&theme.popup_fg))
            };
            list_items.push(ListItem::new(Line::from(vec![Span::styled(
                format!("  {} v{} by {}", name, version, author),
                style,
            )])));
        }
    }

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(t("plugin_title"))
        .style(bg_style);
    let list = List::new(list_items).block(list_block);
    f.render_widget(list, list_area);

    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(t("plugin_details"))
        .style(bg_style);

    let mut detail_lines = Vec::new();
    if !registry.is_empty() {
        if let Some((name, version, desc, author)) = registry.get(cursor_idx) {
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_lbl"), bold_style),
                Span::styled(name.clone(), text_style),
            ]));
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_latest_ver"), bold_style),
                Span::styled(version.clone(), text_style),
            ]));
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_author"), bold_style),
                Span::styled(author.clone(), text_style),
            ]));
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(Span::styled(
                t("plugin_detail_description"),
                bold_style,
            )));
            let max_width = (detail_area.width as usize).saturating_sub(2);
            for line in wrap_text(desc, max_width) {
                detail_lines.push(Line::from(Span::styled(line, text_style)));
            }
        }
    } else {
        detail_lines.push(Line::from(Span::styled(
            t("plugin_no_selected"),
            Style::default().fg(Color::DarkGray),
        )));
    }

    let detail_para = Paragraph::new(detail_lines)
        .block(detail_block)
        .wrap(Wrap { trim: false });
    f.render_widget(detail_para, detail_area);
}
