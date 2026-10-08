//! Directory hotlist and folder shortcut dialog rendering.

use super::super::centered_rect;
use crate::app::input_popup::folder_shortcuts::{SLOT_COUNT, slot_for_row};
use crate::config::bookmarks::HotlistEntry;
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::keybindings::{Action, KeybindingResolver};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn render_hotlist(
    f: &mut Frame,
    theme: &Theme,
    size: Rect,
    entries: &[HotlistEntry],
    cursor_idx: usize,
) {
    let mut lines: Vec<Line> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let text = format!(" {:<20} ->  {} ", e.name, e.path.to_string_lossy());
            row(theme, text, i == cursor_idx)
        })
        .collect();
    if entries.is_empty() {
        lines.push(hint_line(theme, t("hotlist_empty")));
    }
    lines.push(Line::default());
    lines.push(hint_line(theme, t("hotlist_hint")));
    render_box(f, theme, size, t("popup_hotlist"), lines);
}

pub fn render_folder_shortcuts(
    f: &mut Frame,
    theme: &Theme,
    size: Rect,
    shortcuts: &HashMap<u8, PathBuf>,
    resolver: &KeybindingResolver,
    cursor_idx: usize,
) {
    let unassigned = t("folder_shortcut_unassigned");
    let mut lines: Vec<Line> = (0..SLOT_COUNT)
        .map(|i| {
            let slot = slot_for_row(i);
            let chord = resolver
                .key_for_action(Action::GoFolderShortcut(slot))
                .map(|s| s.to_string())
                .unwrap_or_else(|| slot.to_string());
            let target = shortcuts
                .get(&slot)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| unassigned.clone());
            row(
                theme,
                format!(" {chord:<12} ->  {target} "),
                i == cursor_idx,
            )
        })
        .collect();
    lines.push(Line::default());
    lines.push(hint_line(theme, t("folder_shortcuts_hint")));
    render_box(f, theme, size, t("popup_folder_shortcuts"), lines);
}

fn row(theme: &Theme, text: String, is_cursor: bool) -> Line<'static> {
    let style = if is_cursor {
        Style::default()
            .bg(parse_color(&theme.selection_bg))
            .fg(parse_color(&theme.selection_fg))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(parse_color(&theme.popup_fg))
    };
    Line::from(Span::styled(text, style))
}

fn hint_line(theme: &Theme, text: String) -> Line<'static> {
    Line::from(Span::styled(
        text,
        Style::default()
            .fg(parse_color(&theme.popup_fg))
            .add_modifier(Modifier::DIM),
    ))
}

fn render_box(f: &mut Frame, theme: &Theme, size: Rect, title: String, lines: Vec<Line>) {
    let area = centered_rect(60, 40, size);
    f.render_widget(Clear, area);
    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(parse_color(&theme.popup_border)))
            .title(title)
            .style(Style::default().bg(parse_color(&theme.popup_bg))),
    );
    f.render_widget(paragraph, area);
}
