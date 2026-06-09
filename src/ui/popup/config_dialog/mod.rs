pub mod colors;
pub mod confirmations;
pub mod editor_viewer;
pub mod interface;
pub mod panel;
pub mod plugins;
pub mod system;

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
                format!(
                    " {} ",
                    crate::config::localization::t("tab_system")
                ),
                format!(" {} ", crate::config::localization::t("tab_panel")),
                format!(
                    " {} ",
                    crate::config::localization::t("tab_interface")
                ),
                format!(
                    " {} ",
                    crate::config::localization::t("tab_confirmations")
                ),
                format!(
                    " {} ",
                    crate::config::localization::t("tab_plugins")
                ),
                format!(
                    " {} ",
                    crate::config::localization::t("tab_editor")
                ),
                format!(
                    " {} ",
                    crate::config::localization::t("tab_colors")
                ),
            ];
            let mut tab_spans = Vec::new();
            for (i, title) in tab_titles.iter().enumerate() {
                let is_active = i == *active_tab;
                let style = if is_active {
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(parse_color(&theme.popup_fg))
                };
                tab_spans.push(Span::styled(format!("  [{}]  ", title), style));
            }
            f.render_widget(Paragraph::new(Line::from(tab_spans)), header_area);

            let mut rows: Vec<(String, bool)> = Vec::new();

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
                _ => {}
            }

            rows.push((crate::config::localization::t("btn_ok"), false));
            rows.push((crate::config::localization::t("btn_cancel"), false));

            let list_height = content_area.height as usize;
            let scroll_start = cursor_idx.saturating_sub(list_height / 2);
            let mut list_spans = Vec::new();

            for (i, (label, is_stub)) in
                rows.iter().enumerate().skip(scroll_start).take(list_height)
            {
                let is_cursor = i == *cursor_idx;

                let display_label = if *is_stub {
                    format!("{} *", label)
                } else {
                    label.clone()
                };

                let style = if is_cursor {
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD)
                } else if *is_stub {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(parse_color(&theme.popup_fg))
                };

                list_spans.push(Line::from(Span::styled(
                    format!("  {}  ", display_label),
                    style,
                )));
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
