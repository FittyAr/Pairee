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

pub mod wrap;
pub use wrap::wrap_text;

/// Where a plugin-manager tab draws: list and detail areas plus styles.
pub struct Pane<'a> {
    pub list_area: Rect,
    pub detail_area: Rect,
    pub theme: &'a crate::config::theme::Theme,
    pub border_style: Style,
    pub bg_style: Style,
}

/// Rotating spinner character for indeterminate progress (~5 fps).
pub(crate) fn spinner_frame() -> &'static str {
    const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() / 200)
        .unwrap_or(0);
    FRAMES[(now as usize) % FRAMES.len()]
}

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    context: &crate::app::context::AppContext,
) -> bool {
    if let PopupType::PluginMenu(
        menu @ crate::app::state::PluginMenuState {
            active_tab,
            search_query,
            editing_query,
            dev_wizard_step,
            ..
        },
    ) = popup
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
        let pane = Pane {
            list_area: content_chunks[0],
            detail_area: content_chunks[1],
            theme,
            border_style,
            bg_style,
        };

        if *active_tab == 0 {
            installed::render_installed(f, &pane, menu);
        } else if *active_tab == 1 {
            search::render_search(f, &pane, menu);
        } else {
            dev::render_dev(f, &pane, menu, &context.config.settings.active_dev_plugin);
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
