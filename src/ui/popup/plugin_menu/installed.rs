use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier as StyleModifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub fn render_installed(
    f: &mut Frame,
    list_area: Rect,
    detail_area: Rect,
    cursor_idx: usize,
    installed: &[(String, String, bool, bool, Option<String>)],
    theme: &crate::config::theme::Theme,
    border_style: Style,
    bg_style: Style,
) {
    let text_style = Style::default().fg(parse_color(&theme.popup_fg));
    let bold_style = text_style.add_modifier(StyleModifier::BOLD);

    let mut list_items = Vec::new();
    for (i, (name, version, pinned, trusted, update_available)) in installed.iter().enumerate() {
        let pin_badge = if *pinned { " [P]" } else { "" };
        let trust_badge = if *trusted { " [T]" } else { " [U]" };
        let update_badge = if update_available.is_some() {
            " [▲]"
        } else {
            ""
        };

        let style = if i == cursor_idx {
            Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(StyleModifier::BOLD)
        } else {
            Style::default().fg(parse_color(&theme.popup_fg))
        };

        list_items.push(ListItem::new(Line::from(vec![Span::styled(
            format!(
                "  {} v{}{}{}{}",
                name, version, pin_badge, trust_badge, update_badge
            ),
            style,
        )])));
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
    if !installed.is_empty() {
        if let Some((name, version, pinned, trusted, update_available)) = installed.get(cursor_idx)
        {
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_name"), bold_style),
                Span::styled(name.clone(), text_style),
            ]));
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_version"), bold_style),
                Span::styled(version.clone(), text_style),
            ]));
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_trust"), bold_style),
                Span::styled(
                    if *trusted {
                        t("plugin_detail_trusted_desc")
                    } else {
                        t("plugin_detail_untrusted_desc")
                    },
                    text_style,
                ),
            ]));
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_pinned"), bold_style),
                Span::styled(
                    if *pinned {
                        t("plugin_detail_pinned_yes")
                    } else {
                        t("plugin_detail_pinned_no")
                    },
                    text_style,
                ),
            ]));
            if let Some(new_ver) = update_available {
                detail_lines.push(Line::from(vec![
                    Span::styled(
                        t("plugin_detail_update_avail"),
                        bold_style.fg(Color::Yellow),
                    ),
                    Span::styled(
                        format!("v{}{}", new_ver, t("plugin_detail_press_update")),
                        text_style.fg(Color::Yellow),
                    ),
                ]));
            } else {
                detail_lines.push(Line::from(vec![
                    Span::styled(t("plugin_detail_update_status"), bold_style),
                    Span::styled(t("plugin_detail_up_to_date"), text_style),
                ]));
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
