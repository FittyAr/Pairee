use super::state::ViewerState;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface};
use crate::ui::scrollbar::{ScrollTarget, ScrollView};
use crate::ui::search_highlight::highlight_line;
use crate::ui::text_width::expand_tabs;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};

pub(crate) fn render_text(
    f: &mut Frame,
    area: Rect,
    state: &ViewerState,
    block: Block,
    opts: &super::ViewerOpts,
) {
    let super::ViewerOpts {
        theme,
        active_popup,
        show_scrollbar,
        tab_size,
        scrollbar,
    } = *opts;
    let height = area.height.saturating_sub(2) as usize;

    let search_info = match active_popup {
        Some(crate::app::state::PopupType::ViewerSearchPrompt(search)) => search.active_query(),
        _ => None,
    };

    let lines: Vec<Line> = state
        .lines
        .iter()
        .skip(state.scroll)
        .take(height)
        .map(|l| expand_tabs(l, tab_size))
        .map(|l| {
            if let Some((q, cs)) = search_info {
                let normal_style = Style::default().fg(parse_color(&theme.panel_fg));
                let highlight_style = Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.marked_fg))
                    .add_modifier(Modifier::BOLD);
                Line::from(highlight_line(&l, q, cs, normal_style, highlight_style))
            } else {
                Line::from(Span::raw(l))
            }
        })
        .collect();

    let para = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(parse_color(&theme.panel_fg)))
        .wrap(Wrap { trim: false });

    f.render_widget(para, area);

    if show_scrollbar {
        scrollbar::render_vertical_inside_block(
            f,
            area,
            ScrollView {
                content_len: state.lines.len(),
                viewport_len: height,
                offset: state.scroll,
            },
            theme,
            ScrollTarget {
                surface: ScrollbarSurface::Panel,
                hits: scrollbar,
                id: ScrollTargetId::Viewer,
            },
        );
    }
}
