//! Render the command palette popup.

use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::keybindings::{Action, KeybindingResolver};
use crate::ui::popup::kit::FilterListView;
use ratatui::{Frame, layout::Rect};

/// Columns of the action label, before its key.
const LABEL_WIDTH: usize = 46;

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &Theme,
    area: Rect,
    resolver: &KeybindingResolver,
) -> bool {
    let PopupType::CommandPalette {
        query,
        cursor_idx,
        items,
    } = popup
    else {
        return false;
    };
    let title = match resolver.key_for_action(Action::CommandPalette) {
        Some(chord) => format!(" {} ({chord}) ", t("command_palette_title").trim()),
        None => t("command_palette_title"),
    };
    FilterListView {
        max_width: 72,
        title,
        query,
        rows: items
            .iter()
            .map(|(label, action)| {
                let chord = resolver.key_for_action(*action).unwrap_or_default();
                format!("{label:<LABEL_WIDTH$} {chord}")
            })
            .collect(),
        cursor: *cursor_idx,
        hint: None,
    }
    .render(f, area, theme);
    true
}
