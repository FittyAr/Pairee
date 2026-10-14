use crate::app::state::CompareStatus;
use crate::app::state::popup::CompareFoldersResult;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_in;
use crate::ui::popup::kit::{ListPopup, Scroll};
use crate::ui::popup::view::{DialogView, ViewCtx};
use crate::ui::text_width::pad_to_width;
use ratatui::{Frame, style::Style};

impl DialogView for CompareFoldersResult {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let CompareFoldersResult { diff, cursor_idx } = self;
        let rows = diff
            .iter()
            .map(|entry| {
                let (status_key, color) = match entry.status {
                    CompareStatus::OnlyLeft => ("compare_status_only_left", theme.ui.success_alt),
                    CompareStatus::OnlyRight => ("compare_status_only_right", theme.ui.warning_alt),
                    CompareStatus::Different => ("compare_status_different", theme.ui.error),
                    CompareStatus::Equal => ("compare_status_equal", theme.ui.muted),
                };
                (
                    format!(
                        " {} | {} ",
                        pad_to_width(&entry.name, 40),
                        pad_to_width(&t(status_key), 20)
                    ),
                    Style::default().fg(color),
                )
            })
            .collect();
        ListPopup {
            area: centered_rect_in(75, 60, size),
            title: t("compare_results_title"),
            border: theme.ui.warning,
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
