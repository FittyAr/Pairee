use super::state::ViewerState;
use crate::syntax::LineTokens;
use crate::ui::search_highlight::match_ranges;
use crate::ui::styled_line::{Layer, LineView, styled_spans};
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};

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
            // The cursor line reads like a selected one (Ctrl+C copies it
            // when nothing is selected); inside a selection it is the end
            // that moves, marked by an underline.
            let n = first + i;
            let row_style = match state.selection {
                Some(_) if n == state.cursor => {
                    Some(theme.selection().add_modifier(Modifier::UNDERLINED))
                }
                Some(s) if s.contains(n) => Some(theme.selection()),
                None if n == state.cursor => Some(theme.selection()),
                _ => None,
            };
            // Under the syntax colors: they keep their color on the bar.
            let base = row_style.unwrap_or_default();
            let spans = Line::from(styled_spans(line, base, &layers, view));
            match row_style {
                Some(style) => spans.style(style),
                None => spans,
            }
        })
        .collect()
}

/// Where `lines` (the first being line `first`) land in `area` once
/// wrapped, for the mouse (see [`super::TextLayout`]).
pub(crate) fn layout(first: usize, lines: &[Line<'static>], area: Rect) -> super::TextLayout {
    let mut rows = Vec::with_capacity(lines.len());
    let mut top = area.y;
    for (i, line) in lines.iter().enumerate() {
        if top >= area.bottom() {
            break;
        }
        let height = Paragraph::new(line.clone())
            .wrap(Wrap { trim: false })
            .line_count(area.width)
            .max(1);
        let height = u16::try_from(height).unwrap_or(u16::MAX);
        rows.push((first + i, top, height));
        top = top.saturating_add(height);
    }
    super::TextLayout { area, rows }
}
