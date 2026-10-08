//! Render the command palette popup.

use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::ui::popup::kit::FilterListView;
use ratatui::{Frame, layout::Rect};

pub fn render(f: &mut Frame, popup: &PopupType, theme: &Theme, area: Rect) -> bool {
    let PopupType::CommandPalette {
        query,
        cursor_idx,
        items,
    } = popup
    else {
        return false;
    };
    FilterListView {
        max_width: 64,
        title: t("command_palette_title"),
        query,
        rows: items.iter().map(|(label, _)| label.clone()).collect(),
        cursor: *cursor_idx,
        hint: None,
    }
    .render(f, area, theme);
    true
}
