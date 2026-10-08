use super::{Pane, spinner_frame};
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    style::{Color, Modifier as StyleModifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
};

pub fn render_installed(f: &mut Frame, pane: &Pane, menu: &crate::app::state::PluginMenuState) {
    let Pane {
        list_area,
        detail_area,
        theme,
        border_style,
        bg_style,
    } = *pane;
    let cursor_idx = menu.cursor_idx;
    let installed = menu.installed.as_slice();
    let installed_loading = menu.installed_loading;
    let installed_loading_status = menu.installed_loading_status.as_str();
    let text_style = Style::default().fg(parse_color(&theme.popup_fg));
    let bold_style = text_style.add_modifier(StyleModifier::BOLD);

    let mut list_items = Vec::new();
    if installed_loading && installed.is_empty() {
        list_items.push(ListItem::new(Line::from(vec![Span::styled(
            format!("  {} {}", spinner_frame(), installed_loading_status),
            Style::default().fg(Color::Yellow),
        )])));
    } else {
        for (i, plugin) in installed.iter().enumerate() {
            let pin_badge = if plugin.pinned { " [P]" } else { "" };
            let trust_badge = if plugin.trusted { " [T]" } else { " [U]" };
            let update_badge = if plugin.update_available.is_some() {
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
                    plugin.name, plugin.version, pin_badge, trust_badge, update_badge
                ),
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
    if installed_loading && installed.is_empty() {
        // Show a progress gauge at indeterminate ratio + the status line
        // so the user sees something is happening.
        let status = if installed_loading_status.is_empty() {
            t("plugin_dev_progress_loading_index")
        } else {
            installed_loading_status.to_string()
        };
        detail_lines.push(Line::from(Span::styled(
            format!("{} {}", spinner_frame(), status),
            Style::default().fg(Color::Yellow),
        )));
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::NONE))
            .gauge_style(
                Style::default()
                    .fg(parse_color(&theme.selection_bg))
                    .bg(parse_color(&theme.popup_bg)),
            )
            .ratio(0.0)
            .label("");
        f.render_widget(gauge, detail_area);
        let detail_para = Paragraph::new(detail_lines)
            .block(detail_block)
            .wrap(Wrap { trim: false });
        f.render_widget(detail_para, detail_area);
        return;
    }

    if !installed.is_empty() {
        if let Some(plugin) = installed.get(cursor_idx) {
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_name"), bold_style),
                Span::styled(plugin.name.clone(), text_style),
            ]));
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_version"), bold_style),
                Span::styled(plugin.version.clone(), text_style),
            ]));
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_trust"), bold_style),
                Span::styled(
                    if plugin.trusted {
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
                    if plugin.pinned {
                        t("plugin_detail_pinned_yes")
                    } else {
                        t("plugin_detail_pinned_no")
                    },
                    text_style,
                ),
            ]));
            detail_lines.push(Line::from(vec![
                Span::styled(t("plugin_detail_commands"), bold_style),
                Span::styled(
                    if plugin.commands.is_empty() {
                        t("plugin_detail_commands_none")
                    } else {
                        plugin.commands.join(", ")
                    },
                    text_style,
                ),
            ]));
            if let Some(new_ver) = &plugin.update_available {
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
