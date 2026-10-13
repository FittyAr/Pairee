use crate::app::state::CompareStatus;
use crate::app::state::popup::CompareFoldersResult;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_in;
use crate::ui::popup::kit::{ListPopup, Scroll};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    style::{Color, Style},
};

impl DialogView for CompareFoldersResult {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let CompareFoldersResult { diff, cursor_idx } = self;
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
            area: centered_rect_in(75, 60, size),
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
    }
}
