use crate::app::state::PopupType;
use crate::ui::theme_apply::parse_color;
use crate::config::localization::t;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    context: &crate::app::context::AppContext,
) -> bool {
    if let PopupType::PluginMenu {
        active_tab,
        cursor_idx,
        installed,
        registry,
        search_query,
        is_searching,
        editing_query,
        dev_results,
    } = popup
    {
        let area = super::centered_rect(85, 80, size);
        f.render_widget(Clear, area);

        let border_style = Style::default().fg(parse_color(&theme.popup_border));
        let bg_style = Style::default().bg(parse_color(&theme.popup_bg));
        let text_style = Style::default().fg(parse_color(&theme.popup_fg));
        let bold_style = text_style.add_modifier(Modifier::BOLD);

        let dev_mode = context.config.settings.plugins_developer_mode;

        let main_chunks = if *active_tab == 1 || (*active_tab == 2 && *editing_query) {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Tab Bar
                    Constraint::Length(3), // Search Input / Prompt
                    Constraint::Min(1),    // List & Detail Panel
                    Constraint::Length(1), // Legend
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Tab Bar
                    Constraint::Min(1),    // List & Detail Panel
                    Constraint::Length(1), // Legend
                ])
                .split(area)
        };

        let tab_area = main_chunks[0];
        let content_area = if *active_tab == 1 || (*active_tab == 2 && *editing_query) { main_chunks[2] } else { main_chunks[1] };
        let legend_area = if *active_tab == 1 || (*active_tab == 2 && *editing_query) { main_chunks[3] } else { main_chunks[2] };

        let tab_title_installed = t("plugin_tab_installed");
        let tab_title_search = t("plugin_tab_search");
        let tab_title_dev = t("plugin_tab_dev");

        let installed_style = if *active_tab == 0 {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let search_style = if *active_tab == 1 {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let dev_style = if *active_tab == 2 {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let mut tab_spans = vec![
            Span::styled(" [ ", Style::default().fg(Color::DarkGray)),
            Span::styled(tab_title_installed, installed_style),
            Span::styled(" ]  [ ", Style::default().fg(Color::DarkGray)),
            Span::styled(tab_title_search, search_style),
            Span::styled(" ]", Style::default().fg(Color::DarkGray)),
        ];

        if dev_mode {
            tab_spans.push(Span::styled("  [ ", Style::default().fg(Color::DarkGray)));
            tab_spans.push(Span::styled(tab_title_dev, dev_style));
            tab_spans.push(Span::styled(" ]", Style::default().fg(Color::DarkGray)));
        }

        let tabs_line = Line::from(tab_spans);

        let tab_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(t("plugin_manager_title"))
            .style(bg_style);
        f.render_widget(Paragraph::new(tabs_line).block(tab_block), tab_area);

        if *active_tab == 1 {
            let search_area = main_chunks[1];
            let search_text = format!("{}{}|", t("plugin_query"), search_query);
            let search_border_color = if *editing_query {
                Color::Yellow
            } else {
                parse_color(&theme.popup_border)
            };
            let search_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(search_border_color))
                .title(t("plugin_search_repo"))
                .style(bg_style);
            f.render_widget(Paragraph::new(search_text).block(search_block), search_area);
        } else if *active_tab == 2 && *editing_query {
            let search_area = main_chunks[1];
            let search_text = format!("{}{}|", t("plugin_enter_name"), search_query);
            let search_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(t("plugin_init_title"))
                .style(bg_style);
            f.render_widget(Paragraph::new(search_text).block(search_block), search_area);
        }

        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Left list
                Constraint::Percentage(60), // Right detail
            ])
            .split(content_area);
        let list_area = content_chunks[0];
        let detail_area = content_chunks[1];

        let mut list_items = Vec::new();
        if *active_tab == 0 {
            for (i, (name, version, pinned, trusted, update_available)) in installed.iter().enumerate() {
                let pin_badge = if *pinned { " [P]" } else { "" };
                let trust_badge = if *trusted { " [T]" } else { " [U]" };
                let update_badge = if update_available.is_some() { " [▲]" } else { "" };

                let style = if i == *cursor_idx {
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(parse_color(&theme.popup_fg))
                };

                list_items.push(ListItem::new(Line::from(vec![
                    Span::styled(format!("  {} v{}{}{}{}", name, version, pin_badge, trust_badge, update_badge), style),
                ])));
            }
        } else if *active_tab == 1 {
            if *is_searching {
                list_items.push(ListItem::new(Line::from(vec![
                    Span::styled(t("plugin_search_searching"), Style::default().fg(Color::Yellow)),
                ])));
            } else if registry.is_empty() {
                list_items.push(ListItem::new(Line::from(vec![
                    Span::styled(t("plugin_search_no_results"), Style::default().fg(Color::DarkGray)),
                ])));
            } else {
                for (i, (name, version, _, author)) in registry.iter().enumerate() {
                    let style = if i == *cursor_idx {
                        Style::default()
                            .bg(parse_color(&theme.selection_bg))
                            .fg(parse_color(&theme.selection_fg))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(parse_color(&theme.popup_fg))
                    };
                    list_items.push(ListItem::new(Line::from(vec![
                        Span::styled(format!("  {} v{} by {}", name, version, author), style),
                    ])));
                }
            }
        } else {
            let dev_options = [
                "plugin_dev_opt_init",
                "plugin_dev_opt_lint",
                "plugin_dev_opt_package",
                "plugin_dev_opt_submit",
            ];
            for (i, opt_key) in dev_options.iter().enumerate() {
                let opt = t(opt_key);
                let style = if i == *cursor_idx {
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(parse_color(&theme.popup_fg))
                };
                list_items.push(ListItem::new(Line::from(vec![
                    Span::styled(opt, style),
                ])));
            }
        }

        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(if *active_tab == 2 { t("plugin_tools_title") } else { t("plugin_title") })
            .style(bg_style);
        let list = List::new(list_items).block(list_block);
        f.render_widget(list, list_area);

        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(if *active_tab == 2 { t("plugin_action_console") } else { t("plugin_details") })
            .style(bg_style);

        let mut detail_lines = Vec::new();
        let desc_init = t("plugin_dev_desc_init");
        let desc_lint = t("plugin_dev_desc_lint");
        let desc_package = t("plugin_dev_desc_package");
        let desc_submit = t("plugin_dev_desc_submit");
        if *active_tab == 0 && !installed.is_empty() {
            if let Some((name, version, pinned, trusted, update_available)) = installed.get(*cursor_idx) {
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
                    Span::styled(if *trusted { t("plugin_detail_trusted_desc") } else { t("plugin_detail_untrusted_desc") }, text_style),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled(t("plugin_detail_pinned"), bold_style),
                    Span::styled(if *pinned { t("plugin_detail_pinned_yes") } else { t("plugin_detail_pinned_no") }, text_style),
                ]));
                if let Some(new_ver) = update_available {
                    detail_lines.push(Line::from(vec![
                        Span::styled(t("plugin_detail_update_avail"), bold_style.fg(Color::Yellow)),
                        Span::styled(format!("v{}{}", new_ver, t("plugin_detail_press_update")), text_style.fg(Color::Yellow)),
                    ]));
                } else {
                    detail_lines.push(Line::from(vec![
                        Span::styled(t("plugin_detail_update_status"), bold_style),
                        Span::styled(t("plugin_detail_up_to_date"), text_style),
                    ]));
                }
            }
        } else if *active_tab == 1 && !registry.is_empty() {
            if let Some((name, version, desc, author)) = registry.get(*cursor_idx) {
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
                detail_lines.push(Line::from(Span::styled(t("plugin_detail_description"), bold_style)));
                detail_lines.push(Line::from(Span::styled(desc.clone(), text_style)));
            }
        } else if *active_tab == 2 {
            if !dev_results.is_empty() {
                // If we have console outputs from the developer tool execution, show them!
                for line in dev_results.lines() {
                    detail_lines.push(Line::from(Span::styled(line, text_style)));
                }
            } else {
                // Render descriptive hints on what the options do
                match *cursor_idx {
                    0 => {
                        for line in desc_init.lines() {
                            detail_lines.push(Line::from(Span::styled(line, text_style)));
                        }
                    }
                    1 => {
                        for line in desc_lint.lines() {
                            detail_lines.push(Line::from(Span::styled(line, text_style)));
                        }
                    }
                    2 => {
                        for line in desc_package.lines() {
                            detail_lines.push(Line::from(Span::styled(line, text_style)));
                        }
                    }
                    3 => {
                        for line in desc_submit.lines() {
                            detail_lines.push(Line::from(Span::styled(line, text_style)));
                        }
                    }
                    _ => {}
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

        let hint_key = if *active_tab == 0 {
            "plugin_hint_tab0"
        } else if *active_tab == 1 {
            "plugin_hint_tab1"
        } else {
            "plugin_hint_tab2"
        };
        f.render_widget(
            Paragraph::new(t(hint_key))
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            legend_area,
        );

        true
    } else {
        false
    }
}
