//! Drive selection, context menu, and archive commands rendering.

use super::super::{centered_rect, centered_rect_in};
use crate::app::state::ActivePanel;
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::ui::popup::kit::{self, ListPopup, Scroll, marked};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
};
use std::path::Path;

/// The rect of `panel`.
fn panel_rect(panel: ActivePanel, left: Rect, right: Rect) -> Rect {
    match panel {
        ActivePanel::Left => left,
        ActivePanel::Right => right,
    }
}

/// `items` with a `>` marker on the cursor row.
fn marked_rows(items: &[String], cursor: usize, style: Style) -> Vec<(String, Style)> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| (marked(item, i == cursor), style))
        .collect()
}

/// A plain menu popup (no hint, no empty text).
fn menu_popup(
    area: Rect,
    title: String,
    border: Color,
    rows: Vec<(String, Style)>,
    cursor: usize,
) -> ListPopup<'static> {
    ListPopup {
        area,
        title,
        border,
        empty: None,
        header: Vec::new(),
        rows,
        cursor,
        scroll: Scroll::HalfPage,
        hint: None,
        scrollbar: None,
    }
}

pub fn render_drive_select(
    f: &mut Frame,
    theme: &Theme,
    (left_rect, right_rect): (Rect, Rect),
    panel: &ActivePanel,
    drives: &[String],
    cursor_idx: usize,
) {
    let panel_label = match panel {
        ActivePanel::Left => t("menu_left"),
        ActivePanel::Right => t("menu_right"),
    };
    menu_popup(
        centered_rect_in(35, 60, panel_rect(*panel, left_rect, right_rect)),
        t("popup_select_drive").replacen("{}", &panel_label, 1),
        parse_color(&theme.popup_border),
        marked_rows(drives, cursor_idx, kit::popup_fg(theme)),
        cursor_idx,
    )
    .render(f, theme);
}

pub fn render_context_menu(
    f: &mut Frame,
    theme: &Theme,
    (left_rect, right_rect): (Rect, Rect),
    active_panel: ActivePanel,
    items: &[String],
    cursor_idx: usize,
) {
    let height_percent = ((items.len() * 10) as u16).clamp(20, 100);
    menu_popup(
        centered_rect_in(
            50,
            height_percent,
            panel_rect(active_panel, left_rect, right_rect),
        ),
        t("popup_actions"),
        parse_color(&theme.popup_border),
        marked_rows(items, cursor_idx, kit::popup_fg(theme)),
        cursor_idx,
    )
    .render(f, theme);
}

pub fn render_archive_commands_menu(
    f: &mut Frame,
    theme: &Theme,
    size: Rect,
    archive_path: &Path,
    items: &[String],
    cursor_idx: usize,
) {
    let title =
        t("popup_archive_commands").replacen("{}", &crate::fs::file_name_lossy(archive_path), 1);
    ListPopup {
        empty: Some(t("no_archive_commands")),
        hint: Some(t("archive_commands_hint")),
        ..menu_popup(
            centered_rect(60, 45, size),
            title,
            Color::Yellow,
            marked_rows(items, cursor_idx, kit::popup_fg(theme)),
            cursor_idx,
        )
    }
    .render(f, theme);
}
