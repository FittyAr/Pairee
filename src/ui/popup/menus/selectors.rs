//! Drive selection, hotlist, context menu, and archive commands rendering.

use super::super::{centered_rect, centered_rect_in};
use crate::app::state::ActivePanel;
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::path::Path;

pub fn render_drive_select(
    f: &mut Frame,
    theme: &Theme,
    left_rect: Rect,
    right_rect: Rect,
    panel: &ActivePanel,
    drives: &[String],
    cursor_idx: usize,
) {
    let panel_rect = match panel {
        ActivePanel::Left => left_rect,
        ActivePanel::Right => right_rect,
    };
    let area = centered_rect_in(35, 60, panel_rect);
    f.render_widget(Clear, area);

    let mut lines = Vec::new();
    for (i, drive) in drives.iter().enumerate() {
        let is_cursor = i == cursor_idx;
        let line_str = if is_cursor {
            format!(" >  {} ", drive)
        } else {
            format!("    {} ", drive)
        };
        let style = if is_cursor {
            Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(parse_color(&theme.popup_fg))
        };
        lines.push(Line::from(Span::styled(line_str, style)));
    }

    let panel_label = match panel {
        ActivePanel::Left => t("menu_left"),
        ActivePanel::Right => t("menu_right"),
    };
    let title = t("popup_select_drive").replacen("{}", &panel_label, 1);
    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(parse_color(&theme.popup_border)))
            .title(title)
            .style(Style::default().bg(parse_color(&theme.popup_bg))),
    );

    f.render_widget(paragraph, area);
}

pub fn render_hotlist(
    f: &mut Frame,
    theme: &Theme,
    size: Rect,
    bookmarks: &[(String, std::path::PathBuf)],
    cursor_idx: usize,
) {
    let area = centered_rect(60, 40, size);
    f.render_widget(Clear, area);

    let mut lines = Vec::new();
    for (i, (name, path)) in bookmarks.iter().enumerate() {
        let is_cursor = i == cursor_idx;
        let line_str = format!(" {:<20} ->  {} ", name, path.to_string_lossy());
        let style = if is_cursor {
            Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(parse_color(&theme.popup_fg))
        };
        lines.push(Line::from(Span::styled(line_str, style)));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(parse_color(&theme.popup_border)))
            .title(t("popup_hotlist"))
            .style(Style::default().bg(parse_color(&theme.popup_bg))),
    );

    f.render_widget(paragraph, area);
}

pub fn render_context_menu(
    f: &mut Frame,
    theme: &Theme,
    left_rect: Rect,
    right_rect: Rect,
    active_panel: ActivePanel,
    items: &[String],
    cursor_idx: usize,
) {
    let panel_rect = match active_panel {
        ActivePanel::Left => left_rect,
        ActivePanel::Right => right_rect,
    };
    let height_percent = ((items.len() * 10) as u16).clamp(20, 100);
    let area = centered_rect_in(50, height_percent, panel_rect);
    f.render_widget(Clear, area);

    let mut lines = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let is_cursor = i == cursor_idx;
        let line_str = if is_cursor {
            format!(" >  {} ", item)
        } else {
            format!("    {} ", item)
        };
        let style = if is_cursor {
            Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(parse_color(&theme.popup_fg))
        };
        lines.push(Line::from(Span::styled(line_str, style)));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(parse_color(&theme.popup_border)))
            .title(t("popup_actions"))
            .style(Style::default().bg(parse_color(&theme.popup_bg))),
    );
    f.render_widget(paragraph, area);
}

pub fn render_archive_commands_menu(
    f: &mut Frame,
    theme: &Theme,
    size: Rect,
    archive_path: &Path,
    items: &[String],
    cursor_idx: usize,
) {
    let area = centered_rect(60, 45, size);
    f.render_widget(Clear, area);

    let archive_name = archive_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let title = t("popup_archive_commands").replacen("{}", &archive_name, 1);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Yellow))
        .title(title)
        .style(Style::default().bg(parse_color(&theme.popup_bg)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if items.is_empty() {
        let paragraph = Paragraph::new(t("no_archive_commands"))
            .style(Style::default().fg(parse_color(&theme.popup_fg)));
        f.render_widget(paragraph, inner);
    } else {
        let list_height = inner.height.saturating_sub(2) as usize;
        let scroll_start = cursor_idx.saturating_sub(list_height / 2);
        let mut lines = Vec::new();

        for (i, item) in items
            .iter()
            .enumerate()
            .skip(scroll_start)
            .take(list_height)
        {
            let is_cursor = i == cursor_idx;
            let line_str = if is_cursor {
                format!(" >  {} ", item)
            } else {
                format!("    {} ", item)
            };
            let style = if is_cursor {
                Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.selection_fg))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(parse_color(&theme.popup_fg))
            };
            lines.push(Line::from(Span::styled(line_str, style)));
        }

        let hint = Line::from(Span::styled(
            t("archive_commands_hint"),
            Style::default().fg(ratatui::style::Color::DarkGray),
        ));
        lines.push(Line::from(""));
        lines.push(hint);

        let paragraph =
            Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
        f.render_widget(paragraph, inner);
    }
}
