pub mod colors;
pub mod confirmations;
pub mod editor_viewer;
pub mod git;
pub mod interface;
pub mod panel;
pub mod plugins;
pub mod system;

#[derive(Clone, Debug, PartialEq)]
pub enum RowType {
    Setting(usize),
    Title,
    Subtitle,
    Hint,
}

use super::centered_rect;
use crate::app::state::PopupType;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render_config_dialog_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::ConfigurationDialog {
            active_tab,
            cursor_idx,
            editing_value,
            edit_buffer,
            settings,
        } => {
            let area = centered_rect(85, 85, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(crate::config::localization::t("config_dialog_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Tab headers
                    Constraint::Length(1), // Separator
                    Constraint::Min(1),    // Tab contents
                    Constraint::Length(1), // Bottom separator
                    Constraint::Length(1), // Hint/Status bar
                ])
                .split(inner);

            let header_area = chunks[0];
            let separator_area = chunks[1];
            let content_area = chunks[2];
            let bottom_sep_area = chunks[3];
            let hint_area = chunks[4];

            f.render_widget(
                Paragraph::new("─".repeat(inner.width as usize))
                    .style(Style::default().fg(Color::DarkGray)),
                separator_area,
            );
            f.render_widget(
                Paragraph::new("─".repeat(inner.width as usize))
                    .style(Style::default().fg(Color::DarkGray)),
                bottom_sep_area,
            );

            let tab_titles = [
                crate::config::localization::t("tab_system"),
                crate::config::localization::t("tab_panel"),
                crate::config::localization::t("tab_interface"),
                crate::config::localization::t("tab_confirmations"),
                crate::config::localization::t("tab_plugins"),
                crate::config::localization::t("tab_editor"),
                crate::config::localization::t("tab_colors"),
                crate::config::localization::t("tab_git"),
            ];
            let mut tab_spans = Vec::new();
            for (i, title) in tab_titles.iter().enumerate() {
                let is_active = i == *active_tab;
                let base_style = if is_active {
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(parse_color(&theme.popup_fg))
                };
                let hotkey_style = if is_active {
                    base_style.fg(ratatui::style::Color::Yellow)
                } else {
                    base_style
                        .fg(ratatui::style::Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                };

                tab_spans.push(Span::styled("  [ ", base_style));
                let text_spans =
                    crate::ui::hotkey::render_hotkey_spans(title, base_style, hotkey_style);
                tab_spans.extend(text_spans);
                tab_spans.push(Span::styled(" ]", base_style));
            }
            f.render_widget(Paragraph::new(Line::from(tab_spans)), header_area);

            let mut rows: Vec<(String, RowType)> = Vec::new();

            match active_tab {
                0 => system::populate_rows(settings, &mut rows),
                1 => panel::populate_rows(settings, &mut rows),
                2 => interface::populate_rows(
                    settings,
                    *editing_value,
                    *cursor_idx,
                    edit_buffer,
                    &mut rows,
                ),
                3 => confirmations::populate_rows(settings, &mut rows),
                4 => plugins::populate_rows(settings, &mut rows),
                5 => editor_viewer::populate_rows(
                    settings,
                    *editing_value,
                    *cursor_idx,
                    edit_buffer,
                    &mut rows,
                ),
                6 => colors::populate_rows(settings, &mut rows),
                7 => git::populate_rows(
                    settings,
                    *editing_value,
                    *cursor_idx,
                    edit_buffer,
                    &mut rows,
                ),
                _ => {}
            }

            rows.push((
                crate::config::localization::t("btn_ok"),
                RowType::Setting(9998),
            ));
            rows.push((
                crate::config::localization::t("btn_cancel"),
                RowType::Setting(9999),
            ));

            let list_height = content_area.height as usize;
            let scroll_start = cursor_idx.saturating_sub(list_height / 2);
            let mut list_spans = Vec::new();

            for (i, (label, row_type)) in
                rows.iter().enumerate().skip(scroll_start).take(list_height)
            {
                let is_cursor = i == *cursor_idx;

                let style = match row_type {
                    RowType::Title => Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    RowType::Subtitle => Style::default().fg(Color::Yellow),
                    RowType::Hint => Style::default().fg(Color::DarkGray),
                    RowType::Setting(_) => {
                        if is_cursor {
                            Style::default()
                                .bg(parse_color(&theme.selection_bg))
                                .fg(parse_color(&theme.selection_fg))
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(parse_color(&theme.popup_fg))
                        }
                    }
                };

                let display_label = match row_type {
                    RowType::Title => format!("━━━ {} ━━━", label),
                    RowType::Subtitle => format!("  {}", label),
                    RowType::Setting(_) => format!("  {}  ", label),
                    RowType::Hint => format!("  {}  ", label),
                };

                list_spans.push(Line::from(Span::styled(display_label, style)));
            }

            f.render_widget(Paragraph::new(list_spans), content_area);

            let hint_str = crate::config::localization::t("config_dialog_hint");
            let hint_widget = Paragraph::new(hint_str).style(Style::default().fg(Color::Yellow));
            f.render_widget(hint_widget, hint_area);
            true
        }
        _ => false,
    }
}
