use crate::app::state::popup::SelectDevPlugin;
use crate::config::localization::t;
use crate::ui::popup::kit::{ListPopup, Scroll, list_area};
use crate::ui::popup::view::{DialogView, ViewCtx};
use crate::ui::scrollbar::ScrollTargetId;
use ratatui::{Frame, style::Color};

impl DialogView for SelectDevPlugin {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size, scrollbar) = (cx.theme, cx.area, cx.scrollbar());
        let SelectDevPlugin {
            options,
            cursor_idx,
            ..
        } = self;
        let width = options
            .iter()
            .map(|(name, _)| name.len())
            .max()
            .unwrap_or(0);
        ListPopup {
            // Two extra columns for the scrollbar.
            area: list_area(size, width.max(30) + 2, options.len(), 15),
            title: t("dev_plugin_select_title"),
            border: Color::Yellow,
            empty: None,
            header: Vec::new(),
            rows: options
                .iter()
                .map(|(name, _)| (name.clone(), theme.popup_fg()))
                .collect(),
            cursor: *cursor_idx,
            scroll: Scroll::Centered,
            hint: None,
            scrollbar: Some((scrollbar, ScrollTargetId::PluginSelect)),
        }
        .render(f, theme);
    }
}
