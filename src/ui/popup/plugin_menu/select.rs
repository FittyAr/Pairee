use crate::app::state::PopupType;
use crate::ui::popup::kit::{ListPopup, Scroll, list_area};
use crate::ui::scrollbar::{ScrollTargetId, ScrollbarUiState};
use ratatui::{Frame, layout::Rect, style::Color};

pub fn render_dev_select(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
    scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    let PopupType::SelectDevPlugin {
        options,
        cursor_idx,
        ..
    } = popup
    else {
        return false;
    };
    let width = options
        .iter()
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or(0);
    ListPopup {
        // Two extra columns for the scrollbar.
        area: list_area(size, width.max(30) + 2, options.len(), 15),
        title: " Select Active Development Plugin ".to_string(),
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
    true
}
