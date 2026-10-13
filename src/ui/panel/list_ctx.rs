//! What every panel view needs to draw its rows: the panel, focus and the
//! per-row style (cursor, selection, quick-filter dimming, Git status).

use super::helpers::{entry_display_name, entry_display_name_truncated};
use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, PanelState};
use crate::app::text_filter::TextFilter;
use crate::fs::FileEntry;
use crate::ui::highlight::Highlighter;
use crate::ui::hit::{self, HitTarget};
use crate::ui::resolved_theme::ResolvedTheme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::sync::Arc;

pub(crate) struct ListCtx<'a> {
    pub panel: &'a PanelState,
    pub side: ActivePanel,
    /// First shown entry while a file drag scrolls this panel.
    pub peek: Option<usize>,
    pub is_active: bool,
    pub context: &'a AppContext,
    pub theme: Arc<ResolvedTheme>,
    /// Compiled "Files highlighting" rules, when highlighting is on.
    pub highlighter: Option<Arc<Highlighter>>,
    /// Quick filter: entries it does not match are dimmed.
    pub quick_filter: Option<TextFilter>,
}

impl ListCtx<'_> {
    /// Entries shown in `height` rows.
    pub fn range(&self, height: usize) -> (usize, usize) {
        super::helpers::shown_range(self.panel, height, self.peek)
    }

    /// Records the rows drawn in `area`, top down, from entry `first` on.
    pub fn register_rows(&self, area: Rect, first: usize) {
        let shown = self.panel.entries.len().saturating_sub(first);
        for (row, index) in (area.y..area.bottom()).zip(first..first + shown) {
            hit::register(
                Rect::new(area.x, row, area.width, 1),
                HitTarget::PanelRow {
                    side: self.side,
                    index,
                },
            );
        }
    }

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
        match &self.quick_filter {
            Some(filter) if entry.name != ".." => !filter.matches(&entry.name),
            _ => false,
        }
    }

    /// Style of the row showing entry `idx`.
    pub fn row_style(&self, idx: usize, entry: &FileEntry) -> Style {
        let theme = &self.theme;
        let is_cursor = idx == self.panel.cursor_index;
        let is_selected = self.panel.selected_paths.contains(&entry.path);
        let base = Style::default().fg(theme.panel_fg);
        let mut style = match &self.highlighter {
            Some(highlighter) => highlighter.style(entry, base),
            None => base,
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
            style = style.fg(theme.marked_fg);
        }
        match (is_cursor, self.is_active) {
            (true, true) => style.patch(theme.selection()).add_modifier(Modifier::BOLD),
            (true, false) => style.bg(Color::DarkGray),
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
