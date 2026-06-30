use crate::app::state::PopupType;
use crate::ui::theme_apply::parse_color;
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
) -> bool {
    if let PopupType::PluginMenu {
        active_tab,
        cursor_idx,
        installed,
        registry,
        search_query,
        is_searching,
        editing_query,
    } = popup
    {
        let area = super::centered_rect(85, 80, size);
        f.render_widget(Clear, area);

        let border_style = Style::default().fg(parse_color(&theme.popup_border));
        let bg_style = Style::default().bg(parse_color(&theme.popup_bg));
        let text_style = Style::default().fg(parse_color(&theme.popup_fg));
        let bold_style = text_style.add_modifier(Modifier::BOLD);

        let main_chunks = if *active_tab == 1 {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Tab Bar
                    Constraint::Length(3), // Search Input
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
        let content_area = if *active_tab == 1 { main_chunks[2] } else { main_chunks[1] };
        let legend_area = if *active_tab == 1 { main_chunks[3] } else { main_chunks[2] };

        let tab_title_installed = " Installed Plugins ";
        let tab_title_search = " Search Registry ";

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

        let tabs_line = Line::from(vec![
            Span::styled(" [ ", Style::default().fg(Color::DarkGray)),
            Span::styled(tab_title_installed, installed_style),
            Span::styled(" ]  [ ", Style::default().fg(Color::DarkGray)),
            Span::styled(tab_title_search, search_style),
            Span::styled(" ]", Style::default().fg(Color::DarkGray)),
        ]);

        let tab_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Plugins Manager ")
            .style(bg_style);
        f.render_widget(Paragraph::new(tabs_line).block(tab_block), tab_area);

        if *active_tab == 1 {
            let search_area = main_chunks[1];
            let search_text = format!(" Query: {}|", search_query);
            let search_border_color = if *editing_query {
                Color::Yellow
            } else {
                parse_color(&theme.popup_border)
            };
            let search_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(search_border_color))
                .title(" Search Repository ")
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
        } else {
            if *is_searching {
                list_items.push(ListItem::new(Line::from(vec![
                    Span::styled("  Searching...", Style::default().fg(Color::Yellow)),
                ])));
            } else if registry.is_empty() {
                list_items.push(ListItem::new(Line::from(vec![
                    Span::styled("  No results found.", Style::default().fg(Color::DarkGray)),
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
        }

        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Plugins ")
            .style(bg_style);
        let list = List::new(list_items).block(list_block);
        f.render_widget(list, list_area);

        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Details ")
            .style(bg_style);

        let mut detail_lines = Vec::new();
        if *active_tab == 0 && !installed.is_empty() {
            if let Some((name, version, pinned, trusted, update_available)) = installed.get(*cursor_idx) {
                detail_lines.push(Line::from(vec![
                    Span::styled("Plugin Name: ", bold_style),
                    Span::styled(name.clone(), text_style),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Installed Version: ", bold_style),
                    Span::styled(version.clone(), text_style),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Trust Status: ", bold_style),
                    Span::styled(if *trusted { "Trusted (allows commands/system IO)" } else { "Untrusted (fully sandboxed)" }, text_style),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Pinned Version: ", bold_style),
                    Span::styled(if *pinned { "Yes (will not be automatically updated)" } else { "No" }, text_style),
                ]));
                if let Some(new_ver) = update_available {
                    detail_lines.push(Line::from(vec![
                        Span::styled("Update Available: ", bold_style.fg(Color::Yellow)),
                        Span::styled(format!("v{} (Press 'u' to update)", new_ver), text_style.fg(Color::Yellow)),
                    ]));
                } else {
                    detail_lines.push(Line::from(vec![
                        Span::styled("Update Status: ", bold_style),
                        Span::styled("Up to date", text_style),
                    ]));
                }
            }
        } else if *active_tab == 1 && !registry.is_empty() {
            if let Some((name, version, desc, author)) = registry.get(*cursor_idx) {
                detail_lines.push(Line::from(vec![
                    Span::styled("Plugin: ", bold_style),
                    Span::styled(name.clone(), text_style),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Latest Version: ", bold_style),
                    Span::styled(version.clone(), text_style),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Author: ", bold_style),
                    Span::styled(author.clone(), text_style),
                ]));
                detail_lines.push(Line::from(""));
                detail_lines.push(Line::from(Span::styled("Description:", bold_style)));
                detail_lines.push(Line::from(Span::styled(desc.clone(), text_style)));
            }
        } else {
            detail_lines.push(Line::from(Span::styled(
                "No plugin selected.",
                Style::default().fg(Color::DarkGray),
            )));
        }

        let detail_para = Paragraph::new(detail_lines)
            .block(detail_block)
            .wrap(Wrap { trim: false });
        f.render_widget(detail_para, detail_area);

        let hint_text = if *active_tab == 0 {
            " [Tab] Switch Tab  [t] Toggle Trust  [p] Toggle Pin  [u] Update Selected  [U] Update All  [Del/d] Uninstall  [Esc] Close "
        } else {
            " [Tab] Switch Tab  [/] Edit Query  [Enter] Search  [i] Install Selected  [Esc] Close "
        };
        f.render_widget(
            Paragraph::new(hint_text)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            legend_area,
        );

        true
    } else {
        false
    }
}
