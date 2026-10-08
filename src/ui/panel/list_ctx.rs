//! What every panel view needs to draw its rows: the panel, focus and the
//! per-row style (cursor, selection, quick-filter dimming, Git status).

use super::helpers::{entry_display_name, entry_display_name_truncated};
use crate::app::context::AppContext;
use crate::app::state::PanelState;
use crate::fs::FileEntry;
use crate::ui::theme_apply::parse_color;
use ratatui::style::{Color, Modifier, Style};

pub(crate) struct ListCtx<'a> {
    pub panel: &'a PanelState,
    pub is_active: bool,
    pub context: &'a AppContext,
    pub highlight_files: bool,
}

impl ListCtx<'_> {
    /// Git status letter of `entry` (when Git integration is on).
    pub fn git_status(&self, entry: &FileEntry) -> Option<&str> {
        if self.context.config.settings.git_enabled {
            self.panel.git_statuses.get(&entry.name).map(|s| s.as_str())
        } else {
            None
        }
    }

    /// Name cell text, truncated to `width` columns when given.
    pub fn name(&self, entry: &FileEntry, width: Option<usize>) -> String {
        let git = self.git_status(entry);
        match width {
            Some(width) => entry_display_name_truncated(&entry.name, entry.is_dir, width, git),
            None => entry_display_name(&entry.name, entry.is_dir, git),
        }
    }

    /// Entries not matching the quick filter are dimmed (".." never is).
    fn is_dimmed(&self, entry: &FileEntry) -> bool {
        match &self.panel.quick_filter_mask {
            Some(mask) if entry.name != ".." => {
                !entry.name.to_lowercase().contains(&mask.to_lowercase())
            }
            _ => false,
        }
    }

    /// Style of the row showing entry `idx`.
    pub fn row_style(&self, idx: usize, entry: &FileEntry) -> Style {
        let theme = &self.context.config.theme;
        let is_cursor = idx == self.panel.cursor_index;
        let is_selected = self.panel.selected_paths.contains(&entry.path);
        let base = Style::default().fg(parse_color(&theme.panel_fg));
        let mut style = if self.highlight_files {
            let rules = crate::ui::highlight::default_highlight_rules();
            crate::ui::highlight::style_for_entry(entry, &rules, base)
        } else {
            base
        };
        if !is_selected
            && (!is_cursor || !self.is_active)
            && let Some(color) = self.git_status(entry).and_then(git_status_color)
        {
            style = style.fg(color);
        }
        if self.is_dimmed(entry) {
            style = style.fg(Color::DarkGray);
        }
        if is_selected {
            style = style.fg(parse_color(&theme.marked_fg));
        }
        match (is_cursor, self.is_active) {
            (true, true) => style
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(Modifier::BOLD),
            (true, false) => style.bg(parse_color("DarkGray")),
            _ => style,
        }
    }
}

/// Foreground for a Git status letter.
fn git_status_color(status: &str) -> Option<Color> {
    Some(match status {
        "M" => Color::Yellow,
        "A" => Color::Green,
        "?" => Color::Magenta,
        "D" => Color::Red,
        "!" => Color::LightRed,
        _ => return None,
    })
}
