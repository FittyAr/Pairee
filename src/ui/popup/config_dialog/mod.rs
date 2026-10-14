//! Configuration dialog: tab list on the left, the rows of the active tab
//! (from [`crate::app::config_rows`]) on the right.

use super::centered_rect_in;
use crate::app::config_rows::{Row, RowCtx, TAB_KEYS, tab_rows};
use crate::app::state::ConfigurationDialogState;
use crate::config::localization::t;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::ui::resolved_theme::ResolvedTheme;

impl DialogView for ConfigurationDialogState {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size, keybindings) = (cx.theme, cx.area, &cx.context.config.keybindings);
        let dialog = self;
        let area = centered_rect_in(85, 85, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.popup_border))
            .title(t("config_dialog_title"))
            .style(Style::default().bg(theme.popup_bg));

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

        render_separators(f, (body_chunks[1], main_chunks[1]), inner.width, theme);
        f.render_widget(Paragraph::new(tab_lines(dialog, theme)), body_chunks[0]);
        let content_area = body_chunks[2];
        let lines = row_lines(dialog, theme, keybindings, content_area.height as usize);
        f.render_widget(Paragraph::new(lines), content_area);
        register_mouse(dialog, keybindings, body_chunks[0], content_area);

        let hint_widget =
            Paragraph::new(t("config_dialog_hint")).style(Style::default().fg(theme.ui.warning));
        f.render_widget(hint_widget, main_chunks[2]);
    }
}

/// Tabs take a click through their hotkey; a row is reached with the arrows
/// (which skip titles and hints) and used with Space, OK / Cancel with Enter.
fn register_mouse(
    dialog: &ConfigurationDialogState,
    keybindings: &crate::config::keybindings::KeybindingsConfig,
    tabs_area: Rect,
    rows_area: Rect,
) {
    use crate::app::config_rows::is_selectable;
    use crate::ui::popup::kit::mouse;
    use crossterm::event::{KeyCode, KeyEvent};
    // While a row is edited, a click elsewhere commits it first.
    let prefix: Vec<KeyEvent> = if dialog.edit.is_some() {
        mouse::key(KeyCode::Enter)
    } else {
        Vec::new()
    };
    for (i, title) in TAB_KEYS.map(t).iter().enumerate() {
        let y = tabs_area.y + i as u16;
        if let Some(c) = crate::ui::hotkey::parse_hotkey(title).hotkey
            && y < tabs_area.bottom()
        {
            let mut keys = prefix.clone();
            keys.push(KeyEvent::from(KeyCode::Char(c)));
            mouse::keys(Rect::new(tabs_area.x, y, tabs_area.width, 1), keys);
        }
    }
    let rows = tab_rows(
        dialog.active_tab,
        &RowCtx {
            settings: &dialog.draft,
            keybindings,
        },
    );
    let count = rows.len() + 2;
    let start = dialog
        .cursor_idx
        .saturating_sub(rows_area.height as usize / 2);
    // The key handler first moves a cursor left on a title to the next
    // selectable row; count from there.
    let cursor = (dialog.cursor_idx..count)
        .chain(0..dialog.cursor_idx)
        .find(|&j| is_selectable(&rows, j))
        .unwrap_or(dialog.cursor_idx);
    for (y, i) in (rows_area.y..rows_area.bottom()).zip(start..count) {
        let editing_here = dialog.edit.is_some() && i == cursor;
        if !is_selectable(&rows, i) || editing_here {
            continue;
        }
        let between =
            |from: usize, to: usize| (from..to).filter(|&j| is_selectable(&rows, j)).count() as i64;
        let steps = if i >= cursor {
            between(cursor + 1, i + 1)
        } else {
            -between(i, cursor)
        };
        let mut keys = prefix.clone();
        if dialog.focus_on_tabs && dialog.edit.is_none() {
            keys.push(KeyEvent::from(KeyCode::Right));
        }
        keys.extend(mouse::presses(steps, KeyCode::Down, KeyCode::Up));
        keys.push(KeyEvent::from(if i >= rows.len() {
            KeyCode::Enter
        } else {
            KeyCode::Char(' ')
        }));
        mouse::keys(Rect::new(rows_area.x, y, rows_area.width, 1), keys);
    }
}

/// Vertical line between tabs and rows, horizontal line above the hint.
fn render_separators(
    f: &mut Frame,
    (vertical, bottom): (Rect, Rect),
    width: u16,
    theme: &ResolvedTheme,
) {
    let sep_lines: Vec<Line> = (0..vertical.height)
        .map(|_| Line::from(Span::styled("│", Style::default().fg(theme.ui.muted))))
        .collect();
    f.render_widget(Paragraph::new(sep_lines), vertical);
    f.render_widget(
        crate::ui::popup::kit::separator(width, crate::ui::popup::kit::fg(theme.ui.muted)),
        bottom,
    );
}

/// One `[ Tab ]` line per tab, marking the active one (filled when focused).
fn tab_lines(dialog: &ConfigurationDialogState, theme: &ResolvedTheme) -> Vec<Line<'static>> {
    let focus_on_tabs = dialog.focus_on_tabs;
    TAB_KEYS
        .map(t)
        .iter()
        .enumerate()
        .map(|(i, title)| {
            let is_active = i == dialog.active_tab;
            let (prefix, base_style) = if is_active && focus_on_tabs {
                ("▶ ", theme.selection().add_modifier(Modifier::BOLD))
            } else if is_active {
                (
                    "▷ ",
                    Style::default()
                        .fg(theme.selection_bg)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ("  ", Style::default().fg(theme.popup_fg))
            };
            let hotkey_style = if is_active && focus_on_tabs {
                base_style.fg(theme.ui.warning)
            } else {
                base_style.fg(theme.ui.warning).add_modifier(Modifier::BOLD)
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
    theme: &ResolvedTheme,
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
            .fg(theme.selection_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        theme.selection().add_modifier(Modifier::BOLD)
    };
    let normal = Style::default().fg(theme.popup_fg);

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
                        .fg(theme.ui.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Some(Row::Subtitle(label)) => (
                    format!("  {}", label.text()),
                    Style::default().fg(theme.ui.warning),
                ),
                Some(Row::Hint(label)) => (
                    format!("  {}  ", label.text()),
                    Style::default().fg(theme.ui.muted),
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
