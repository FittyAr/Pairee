//! Render the command palette popup.

use crate::app::state::popup::CommandPalette;
use crate::config::localization::t;
use crate::keybindings::Action;
use crate::ui::popup::kit::FilterListView;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::Frame;

/// Columns of the action label, before its key.
const LABEL_WIDTH: usize = 46;

impl DialogView for CommandPalette {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, area, resolver) = (cx.theme, cx.area, &cx.context.resolver);
        let CommandPalette {
            query,
            cursor_idx,
            items,
        } = self;
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
    }
}
