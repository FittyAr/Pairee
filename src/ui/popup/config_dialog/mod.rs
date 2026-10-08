//! Configuration dialog: tab list on the left, the rows of the active tab
//! (from [`crate::app::config_rows`]) on the right.

use super::centered_rect;
use crate::app::config_rows::{Row, RowCtx, TAB_KEYS, tab_rows};
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
    custom_bindings: &std::collections::HashMap<String, String>,
) -> bool {
    match popup {
        PopupType::ConfigurationDialog(crate::app::state::ConfigurationDialogState {
            active_tab,
            cursor_idx,
            edit,
            settings,
            focus_on_tabs,
        }) => {
            let area = centered_rect(85, 85, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(crate::config::localization::t("config_dialog_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),    // Body area (tabs and contents side by side)
                    Constraint::Length(1), // Bottom separator
                    Constraint::Length(1), // Hint/Status bar
                ])
                .split(inner);

            let body_area = main_chunks[0];
            let bottom_sep_area = main_chunks[1];
            let hint_area = main_chunks[2];

            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(25), // Left panel: Tabs (vertical list)
                    Constraint::Length(1),  // Vertical separator
                    Constraint::Min(1),     // Right panel: Content list
                ])
                .split(body_area);

            let tabs_area = body_chunks[0];
            let vertical_sep_area = body_chunks[1];
            let content_area = body_chunks[2];

            // Render vertical separator
            let sep_height = vertical_sep_area.height as usize;
            let sep_lines: Vec<Line> = (0..sep_height)
                .map(|_| Line::from(Span::styled("│", Style::default().fg(Color::DarkGray))))
                .collect();
            f.render_widget(Paragraph::new(sep_lines), vertical_sep_area);

            // Render horizontal bottom separator
            f.render_widget(
                Paragraph::new("─".repeat(inner.width as usize))
                    .style(Style::default().fg(Color::DarkGray)),
                bottom_sep_area,
            );

            let tab_titles = TAB_KEYS.map(crate::config::localization::t);

            let mut tab_lines = Vec::new();
            for (i, title) in tab_titles.iter().enumerate() {
                let is_active = i == *active_tab;

                let (prefix, base_style) = if is_active && *focus_on_tabs {
                    (
                        "▶ ",
                        Style::default()
                            .bg(parse_color(&theme.selection_bg))
                            .fg(parse_color(&theme.selection_fg))
                            .add_modifier(Modifier::BOLD),
                    )
                } else if is_active && !*focus_on_tabs {
                    (
                        "▷ ",
                        Style::default()
                            .fg(parse_color(&theme.selection_bg))
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ("  ", Style::default().fg(parse_color(&theme.popup_fg)))
                };

                let hotkey_style = if is_active && *focus_on_tabs {
                    base_style.fg(ratatui::style::Color::Yellow)
                } else {
                    base_style
                        .fg(ratatui::style::Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                };

                let mut spans = Vec::new();
                spans.push(Span::styled(prefix, base_style));
                spans.push(Span::styled("[ ", base_style));
                let text_spans =
                    crate::ui::hotkey::render_hotkey_spans(title, base_style, hotkey_style);
                spans.extend(text_spans);
                spans.push(Span::styled(" ]", base_style));

                tab_lines.push(Line::from(spans));
            }
            f.render_widget(Paragraph::new(tab_lines), tabs_area);

            let rows = tab_rows(
                *active_tab,
                &RowCtx {
                    settings,
                    custom_bindings,
                },
            );
            let ok_cancel = [
                crate::config::localization::t("btn_ok"),
                crate::config::localization::t("btn_cancel"),
            ];
            let row_count = rows.len() + ok_cancel.len();

            let list_height = content_area.height as usize;
            let scroll_start = cursor_idx.saturating_sub(list_height / 2);
            let mut list_spans = Vec::new();

            for i in (scroll_start..row_count).take(list_height) {
                let is_cursor = i == *cursor_idx;
                let selected = if *focus_on_tabs {
                    Style::default()
                        .fg(parse_color(&theme.selection_bg))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD)
                };
                let setting_style = if is_cursor {
                    selected
                } else {
                    Style::default().fg(parse_color(&theme.popup_fg))
                };
                let (text, style) = match rows.get(i) {
                    Some(Row::Title(label)) => (
                        format!("━━━ {} ━━━", label.text()),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Some(Row::Subtitle(label)) => (
                        format!("  {}", label.text()),
                        Style::default().fg(Color::Yellow),
                    ),
                    Some(Row::Hint(label)) => (
                        format!("  {}  ", label.text()),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Some(Row::Setting(setting)) => {
                        let editing = edit.as_ref().filter(|_| is_cursor);
                        (
                            format!("  {}  ", setting.text(settings, editing)),
                            setting_style,
                        )
                    }
                    None => (format!("  {}  ", ok_cancel[i - rows.len()]), setting_style),
                };
                list_spans.push(Line::from(Span::styled(text, style)));
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
