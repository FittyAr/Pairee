//! Render the which-key overlay (live keymap chords).

use crate::app::actions::which_key::filter_items;
use crate::app::context::AppContext;
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::keybindings::Action;
use crate::ui::popup::kit::FilterListView;
use ratatui::{Frame, layout::Rect};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &Theme,
    area: Rect,
    context: &AppContext,
) -> bool {
    let PopupType::WhichKey {
        query,
        cursor_idx,
        items,
    } = popup
    else {
        return false;
    };
    let chord = context
        .resolver
        .key_for_action(Action::WhichKey)
        .unwrap_or("Ctrl+Shift+k");
    FilterListView {
        max_width: 72,
        title: format!(" {} ({chord}) ", t("which_key_title")),
        query,
        rows: filter_items(query.text(), items)
            .iter()
            .map(|(bind, label, _)| format!("{bind:<16} {label}"))
            .collect(),
        cursor: *cursor_idx,
        hint: Some(t("which_key_hint")),
    }
    .render(f, area, theme);
    true
}
