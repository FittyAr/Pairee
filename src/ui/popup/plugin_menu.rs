use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            lines.push(String::new());
            continue;
        }
        
        let char_count = line.chars().count();
        if char_count <= max_width {
            lines.push(line.to_string());
        } else {
            let mut current_line = String::new();
            let mut current_len = 0;
            for word in line.split(' ') {
                let word_len = word.chars().count();
                if current_line.is_empty() {
                    current_line = word.to_string();
                    current_len = word_len;
                } else if current_len + 1 + word_len <= max_width {
                    current_line.push(' ');
                    current_line.push_str(word);
                    current_len += 1 + word_len;
                } else {
                    lines.push(current_line);
                    current_line = word.to_string();
                    current_len = word_len;
                }
                
                while current_len > max_width {
                    let chars: Vec<char> = current_line.chars().collect();
                    let head: String = chars[..max_width].iter().collect();
                    let tail: String = chars[max_width..].iter().collect();
                    lines.push(head);
                    current_line = tail;
                    current_len = current_line.chars().count();
                }
            }
            if !current_line.is_empty() {
                lines.push(current_line);
            }
        }
    }
    lines
}

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
        dev_wizard_step,
        dev_wizard_data: _,
    } = popup
    {
        let area = super::centered_rect(85, 80, size);
        f.render_widget(Clear, area);

        let border_style = Style::default().fg(parse_color(&theme.popup_border));
        let bg_style = Style::default().bg(parse_color(&theme.popup_bg));
        f.render_widget(Block::default().style(bg_style), area);
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
        let content_area = if *active_tab == 1 || (*active_tab == 2 && *editing_query) {
            main_chunks[2]
        } else {
            main_chunks[1]
        };
        let legend_area = if *active_tab == 1 || (*active_tab == 2 && *editing_query) {
            main_chunks[3]
        } else {
            main_chunks[2]
        };

        let tab_title_installed = t("plugin_tab_installed");
        let tab_title_search = t("plugin_tab_search");
        let tab_title_dev = t("plugin_tab_dev");

        let installed_style = if *active_tab == 0 {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let search_style = if *active_tab == 1 {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let dev_style = if *active_tab == 2 {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
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
            let search_text = match *dev_wizard_step {
                1 => format!("{}{}|", t("plugin_enter_name"), search_query),
                2 => format!("{}{}|", t("plugin_enter_desc"), search_query),
                3 => format!("{}{}|", t("plugin_enter_author"), search_query),
                5 => format!("{}{}|", t("plugin_enter_commit_desc"), search_query),
                6 => format!("{}{}|", t("plugin_enter_token_optional"), search_query),
                _ => format!("{}{}|", t("plugin_enter_name"), search_query),
            };
            let search_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(if *dev_wizard_step == 5 || *dev_wizard_step == 6 {
                    t("plugin_dev_opt_submit")
                } else {
                    t("plugin_init_title")
                })
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
            for (i, (name, version, pinned, trusted, update_available)) in
                installed.iter().enumerate()
            {
                let pin_badge = if *pinned { " [P]" } else { "" };
                let trust_badge = if *trusted { " [T]" } else { " [U]" };
                let update_badge = if update_available.is_some() {
                    " [▲]"
                } else {
                    ""
                };

                let style = if i == *cursor_idx {
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD)
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
        } else if *active_tab == 1 {
            if *is_searching {
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
                    let style = if i == *cursor_idx {
                        Style::default()
                            .bg(parse_color(&theme.selection_bg))
                            .fg(parse_color(&theme.selection_fg))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(parse_color(&theme.popup_fg))
                    };
                    list_items.push(ListItem::new(Line::from(vec![Span::styled(
                        format!("  {} v{} by {}", name, version, author),
                        style,
                    )])));
                }
            }
        } else {
            let dev_options = [
                "plugin_dev_opt_init",
                "plugin_dev_opt_lint",
                "plugin_dev_opt_package",
                "plugin_dev_opt_install",
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
                list_items.push(ListItem::new(Line::from(vec![Span::styled(opt, style)])));
            }
        }

        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(if *active_tab == 2 {
                t("plugin_tools_title")
            } else {
                t("plugin_title")
            })
            .style(bg_style);
        let list = List::new(list_items).block(list_block);
        f.render_widget(list, list_area);

        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(if *active_tab == 2 {
                t("plugin_action_console")
            } else {
                t("plugin_details")
            })
            .style(bg_style);

        let mut detail_lines = Vec::new();
        let desc_init = t("plugin_dev_desc_init");
        let desc_lint = t("plugin_dev_desc_lint");
        let desc_package = t("plugin_dev_desc_package");
        let desc_install = t("plugin_dev_desc_install");
        let desc_submit = t("plugin_dev_desc_submit");
        if *active_tab == 0 && !installed.is_empty() {
            if let Some((name, version, pinned, trusted, update_available)) =
                installed.get(*cursor_idx)
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
                detail_lines.push(Line::from(Span::styled(
                    t("plugin_detail_description"),
                    bold_style,
                )));
                let max_width = (detail_area.width as usize).saturating_sub(2);
                for line in wrap_text(desc, max_width) {
                    detail_lines.push(Line::from(Span::styled(line, text_style)));
                }
            }
        } else if *active_tab == 2 {
            let max_width = (detail_area.width as usize).saturating_sub(2);
            if !dev_results.is_empty() {
                // If we have console outputs from the developer tool execution, show them!
                for line in wrap_text(dev_results, max_width) {
                    detail_lines.push(Line::from(Span::styled(line, text_style)));
                }
            } else {
                // Render descriptive hints on what the options do
                let hint = match *cursor_idx {
                    0 => desc_init,
                    1 => desc_lint,
                    2 => desc_package,
                    3 => desc_install,
                    4 => desc_submit,
                    _ => String::new(),
                };
                for line in wrap_text(&hint, max_width) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text_utf8() {
        let text = "Realiza auditorías de verificación de cumplimiento";
        // Let's wrap it to 15 characters
        let lines = wrap_text(text, 15);
        // It shouldn't panic, and should wrap correctly
        assert!(!lines.is_empty());
        for line in &lines {
            assert!(line.chars().count() <= 15);
        }
    }
}
