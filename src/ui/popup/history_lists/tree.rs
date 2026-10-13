use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_in;
use crate::ui::popup::kit::{ListPopup, Scroll};
use crate::ui::scrollbar::ScrollbarUiState;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
};

pub(super) fn render_tree(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
    _scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    let PopupType::TreeView {
        nodes, cursor_idx, ..
    } = popup
    else {
        return false;
    };
    let rows = nodes
        .iter()
        .map(|node| {
            let prefix = if node.is_dir { "▶ " } else { "  " };
            let style = if node.is_dir {
                Style::default().fg(Color::LightBlue)
            } else {
                theme.popup_fg()
            };
            (
                format!("{}{}{}", "  ".repeat(node.depth), prefix, node.name),
                style,
            )
        })
        .collect();
    ListPopup {
        area: centered_rect_in(55, 70, size),
        title: t("tree_view_title"),
        border: Color::Green,
        empty: None,
        header: Vec::new(),
        rows,
        cursor: *cursor_idx,
        scroll: Scroll::HalfPage,
        hint: Some(t("tree_view_hint")),
        scrollbar: None,
    }
    .render(f, theme);
    true
}
