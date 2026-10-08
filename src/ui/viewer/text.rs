use super::state::ViewerState;
use crate::ui::search_highlight::highlight_line;
use crate::ui::text_width::expand_tabs;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// The `height` visible lines, tabs expanded and search matches highlighted
/// while the find dialog is open.
pub(crate) fn text_lines(
    state: &ViewerState,
    height: usize,
    opts: &super::ViewerOpts,
) -> Vec<Line<'static>> {
    let theme = opts.theme;
    let search_info = match opts.active_popup {
        Some(crate::app::state::PopupType::ViewerSearchPrompt(search)) => search.active_query(),
        _ => None,
    };
    state
        .doc
        .lines(state.scroll as u64, height)
        .iter()
        .map(|l| expand_tabs(l, opts.tab_size))
        .map(|l| match search_info {
            Some((q, cs)) => {
                let normal_style = Style::default().fg(parse_color(&theme.panel_fg));
                let highlight_style = Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.marked_fg))
                    .add_modifier(Modifier::BOLD);
                Line::from(highlight_line(&l, q, cs, normal_style, highlight_style))
            }
            None => Line::from(Span::raw(l)),
        })
        .collect()
}
