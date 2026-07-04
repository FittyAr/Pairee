use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub mod dev;
pub mod installed;
pub mod search;
pub mod select;

pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
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
        all_registry: _,
        registry,
        search_query,
        is_searching,
        editing_query,
        dev_results,
        dev_wizard_step,
        dev_wizard_data: _,
        installed_loading,
        installed_loading_status,
        dev_loading,
        dev_loading_status,
        dev_loading_progress,
    } = popup
    {
        let area = super::centered_rect(85, 80, size);
        f.render_widget(Clear, area);

        let border_style = Style::default().fg(parse_color(&theme.popup_border));
        let bg_style = Style::default().bg(parse_color(&theme.popup_bg));
        f.render_widget(Block::default().style(bg_style), area);

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
                10 => format!("{}{}|", t("plugin_enter_active_dev"), search_query),
                _ => format!("{}{}|", t("plugin_enter_name"), search_query),
            };
            let search_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(if *dev_wizard_step == 5 || *dev_wizard_step == 6 {
                    t("plugin_dev_opt_submit")
                } else if *dev_wizard_step == 10 {
                    t("plugin_dev_opt_active").replace("{}", "")
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

        if *active_tab == 0 {
            installed::render_installed(
                f,
                list_area,
                detail_area,
                *cursor_idx,
                installed,
                *installed_loading,
                installed_loading_status,
                theme,
                border_style,
                bg_style,
            );
        } else if *active_tab == 1 {
            search::render_search(
                f,
                list_area,
                detail_area,
                *cursor_idx,
                registry,
                *is_searching,
                *editing_query,
                theme,
                border_style,
                bg_style,
            );
        } else {
            dev::render_dev(
                f,
                list_area,
                detail_area,
                *cursor_idx,
                dev_results,
                *dev_loading,
                dev_loading_status,
                *dev_loading_progress,
                theme,
                border_style,
                bg_style,
                &context.config.settings.active_dev_plugin,
            );
        }

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
        let lines = wrap_text(text, 15);
        assert!(!lines.is_empty());
        for line in &lines {
            assert!(line.chars().count() <= 15);
        }
    }
}
