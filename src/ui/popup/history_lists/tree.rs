use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect;
use crate::ui::popup::kit::{self, ListPopup, Scroll};
use crate::ui::scrollbar::ScrollbarUiState;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
};

pub(super) fn render_tree(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
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
                kit::popup_fg(theme)
            };
            (
                format!("{}{}{}", "  ".repeat(node.depth), prefix, node.name),
                style,
            )
        })
        .collect();
    ListPopup {
        area: centered_rect(55, 70, size),
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
