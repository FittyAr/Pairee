use crate::app::state::PanelState;
use crate::config::theme::Theme;
use crate::fs::FileEntry;
use crate::ui::theme_apply::parse_color;
use ratatui::style::{Modifier, Style};
use std::path::Path;
use std::time::SystemTime;

pub(crate) fn visible_range(panel: &PanelState, height: usize) -> (usize, usize) {
    let start = if panel.cursor_index > height / 2 {
        panel.cursor_index.saturating_sub(height / 2)
    } else {
        0
    };
    let start = if start + height > panel.entries.len() {
        panel.entries.len().saturating_sub(height)
    } else {
        start
    };
    (
        start,
        start + height.min(panel.entries.len().saturating_sub(start)),
    )
}

pub(crate) fn build_row_style(
    entry: &FileEntry,
    is_cursor: bool,
    is_selected: bool,
    is_active: bool,
    theme: &Theme,
    highlight_files: bool,
    is_dimmed: bool,
) -> Style {
    let base_style = Style::default().fg(parse_color(&theme.panel_fg));
    let mut style = if highlight_files {
        let rules = crate::ui::highlight::default_highlight_rules();
        crate::ui::highlight::style_for_entry(entry, &rules, base_style)
    } else {
        base_style
    };
    if is_dimmed {
        style = style.fg(ratatui::style::Color::DarkGray);
    }
    if is_selected {
        style = style.fg(parse_color(&theme.marked_fg));
    }
    if is_cursor && is_active {
        style = style
            .bg(parse_color(&theme.selection_bg))
            .fg(parse_color(&theme.selection_fg))
            .add_modifier(Modifier::BOLD);
    } else if is_cursor && !is_active {
        style = style.bg(parse_color("DarkGray"));
    }
    style
}

pub(crate) fn entry_display_name(name: &str, is_dir: bool) -> String {
    if is_dir && name != ".." {
        format!("/{}", name)
    } else {
        name.to_string()
    }
}

pub(crate) fn format_file_size(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub(crate) fn format_date(time: Option<SystemTime>) -> String {
    match time {
        Some(t) => {
            let dt: chrono::DateTime<chrono::Local> = t.into();
            dt.format("%d/%m/%Y %H:%M").to_string()
        }
        None => String::new(),
    }
}

pub(crate) fn get_free_space_text(path: &Path) -> String {
    match crate::app::sys_helpers::get_free_space(path) {
        Some(bytes) => format_file_size(bytes),
        None => "?".to_string(),
    }
}
