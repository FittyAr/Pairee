use crate::app::state::{CompareStatus, PopupType};
use crate::config::localization::t;
use crate::ui::popup::centered_rect;
use crate::ui::popup::kit::{ListPopup, Scroll};
use crate::ui::scrollbar::ScrollbarUiState;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
};

pub(super) fn render_compare(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    _scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    let PopupType::CompareFoldersResult { diff, cursor_idx } = popup else {
        return false;
    };
    let rows = diff
        .iter()
        .map(|entry| {
            let (status_key, color) = match entry.status {
                CompareStatus::OnlyLeft => ("compare_status_only_left", Color::LightGreen),
                CompareStatus::OnlyRight => ("compare_status_only_right", Color::LightYellow),
                CompareStatus::Different => ("compare_status_different", Color::LightRed),
                CompareStatus::Equal => ("compare_status_equal", Color::DarkGray),
            };
            (
                format!(" {:<40} | {:<20} ", entry.name, t(status_key)),
                Style::default().fg(color),
            )
        })
        .collect();
    ListPopup {
        area: centered_rect(75, 60, size),
        title: t("compare_results_title"),
        border: Color::Yellow,
        empty: Some(t("compare_results_empty")),
        header: Vec::new(),
        rows,
        cursor: *cursor_idx,
        scroll: Scroll::HalfPage,
        hint: Some(t("compare_results_hint")),
        scrollbar: None,
    }
    .render(f, theme);
    true
}
