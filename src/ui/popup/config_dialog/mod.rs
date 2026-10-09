//! Configuration dialog: tab list on the left, the rows of the active tab
//! (from [`crate::app::config_rows`]) on the right.

use super::centered_rect;
use crate::app::config_rows::{Row, RowCtx, TAB_KEYS, tab_rows};
use crate::app::state::{ConfigurationDialogState, PopupType};
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

type Theme = crate::config::theme::Theme;

pub fn render_config_dialog_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &Theme,
    size: Rect,
    keybindings: &crate::config::keybindings::KeybindingsConfig,
) -> bool {
    let PopupType::ConfigurationDialog(dialog) = popup else {
        return false;
    };
    let area = centered_rect(85, 85, size);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(parse_color(&theme.popup_border)))
        .title(t("config_dialog_title"))
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

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(25), // Left panel: Tabs (vertical list)
            Constraint::Length(1),  // Vertical separator
            Constraint::Min(1),     // Right panel: Content list
        ])
        .split(main_chunks[0]);

    render_separators(f, body_chunks[1], main_chunks[1], inner.width);
    f.render_widget(Paragraph::new(tab_lines(dialog, theme)), body_chunks[0]);
    let content_area = body_chunks[2];
    let lines = row_lines(dialog, theme, keybindings, content_area.height as usize);
    f.render_widget(Paragraph::new(lines), content_area);

    let hint_widget =
        Paragraph::new(t("config_dialog_hint")).style(Style::default().fg(Color::Yellow));
    f.render_widget(hint_widget, main_chunks[2]);
    true
}

/// Vertical line between tabs and rows, horizontal line above the hint.
fn render_separators(f: &mut Frame, vertical: Rect, bottom: Rect, width: u16) {
    let sep_lines: Vec<Line> = (0..vertical.height)
        .map(|_| Line::from(Span::styled("│", Style::default().fg(Color::DarkGray))))
        .collect();
    f.render_widget(Paragraph::new(sep_lines), vertical);
    f.render_widget(
        Paragraph::new("─".repeat(width as usize)).style(Style::default().fg(Color::DarkGray)),
        bottom,
    );
}

/// One `[ Tab ]` line per tab, marking the active one (filled when focused).
fn tab_lines(dialog: &ConfigurationDialogState, theme: &Theme) -> Vec<Line<'static>> {
    let focus_on_tabs = dialog.focus_on_tabs;
    TAB_KEYS
        .map(t)
        .iter()
        .enumerate()
        .map(|(i, title)| {
            let is_active = i == dialog.active_tab;
            let (prefix, base_style) = if is_active && focus_on_tabs {
                (
                    "▶ ",
                    Style::default()
                        .bg(parse_color(&theme.selection_bg))
                        .fg(parse_color(&theme.selection_fg))
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_active {
                (
                    "▷ ",
                    Style::default()
                        .fg(parse_color(&theme.selection_bg))
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ("  ", Style::default().fg(parse_color(&theme.popup_fg)))
            };
            let hotkey_style = if is_active && focus_on_tabs {
                base_style.fg(Color::Yellow)
            } else {
                base_style.fg(Color::Yellow).add_modifier(Modifier::BOLD)
            };

            let mut spans = vec![
                Span::styled(prefix, base_style),
                Span::styled("[ ", base_style),
            ];
            spans.extend(crate::ui::hotkey::render_hotkey_spans(
                title,
                base_style,
                hotkey_style,
            ));
            spans.push(Span::styled(" ]", base_style));
            Line::from(spans)
        })
        .collect()
}

/// The visible rows of the active tab plus OK/Cancel, scrolled around the cursor.
fn row_lines(
    dialog: &ConfigurationDialogState,
    theme: &Theme,
    keybindings: &crate::config::keybindings::KeybindingsConfig,
    list_height: usize,
) -> Vec<Line<'static>> {
    let settings = &dialog.draft;
    let rows = tab_rows(
        dialog.active_tab,
        &RowCtx {
            settings,
            keybindings,
        },
    );
    let ok_cancel = [t("btn_ok"), t("btn_cancel")];
    let row_count = rows.len() + ok_cancel.len();
    let selected = if dialog.focus_on_tabs {
        Style::default()
            .fg(parse_color(&theme.selection_bg))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .bg(parse_color(&theme.selection_bg))
            .fg(parse_color(&theme.selection_fg))
            .add_modifier(Modifier::BOLD)
    };
    let normal = Style::default().fg(parse_color(&theme.popup_fg));

    let scroll_start = dialog.cursor_idx.saturating_sub(list_height / 2);
    (scroll_start..row_count)
        .take(list_height)
        .map(|i| {
            let is_cursor = i == dialog.cursor_idx;
            let setting_style = if is_cursor { selected } else { normal };
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
                    let editing = dialog.edit.as_ref().filter(|_| is_cursor);
                    (
                        format!("  {}  ", setting.text(settings, editing)),
                        setting_style,
                    )
                }
                None => (format!("  {}  ", ok_cancel[i - rows.len()]), setting_style),
            };
            Line::from(Span::styled(text, style))
        })
        .collect()
}
