use super::state::ViewerState;
use crate::syntax::LineTokens;
use crate::ui::search_highlight::match_ranges;
use crate::ui::styled_line::{Layer, LineView, styled_spans};
use ratatui::{style::Style, text::Line};

/// The `height` visible lines: tabs expanded, colored by syntax (when on)
/// and with the search matches highlighted while the find dialog is open.
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
    let first = state.scroll;
    let lines = state.doc.lines(first as u64, height);
    let mut syntax = state.syntax.borrow_mut();
    let tokens: &[LineTokens] = match syntax.as_mut().filter(|_| opts.syntax) {
        Some(cache) => cache.highlight(first..first + lines.len(), |r| {
            state.doc.lines(r.start as u64, r.len())
        }),
        None => &[],
    };
    let view = LineView {
        tab_size: opts.tab_size,
        scroll_x: 0,
        trailing: None,
    };
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let mut layers: Vec<Layer> = tokens
                .get(i)
                .into_iter()
                .flatten()
                .map(|t| (t.range.clone(), theme.syntax(t.kind)))
                .collect();
            if let Some((query, case)) = search_info {
                let found = match_ranges(line, query, case);
                layers.extend(found.into_iter().map(|r| (r, theme.search_match())));
            }
            Line::from(styled_spans(line, Style::default(), &layers, view))
        })
        .collect()
}
