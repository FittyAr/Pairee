use super::layout::{lengths, should_show, track_area_inside_block, track_area_right};
use super::types::{ScrollTargetId, ScrollbarHitTarget, ScrollbarSurface, ScrollbarUiState};
use crate::config::theme::Theme;
use crate::ui::theme_apply::parse_color;
use ratatui::{Frame, layout::Rect, style::Style};
use tui_scrollbar::{GlyphSet, ScrollBar, ScrollBarArrows, TrackClickBehavior};

/// Build a themed vertical [`ScrollBar`], or `None` when content fits the viewport.
pub fn vertical_bar(
    content_len: usize,
    viewport_len: usize,
    offset: usize,
    theme: &Theme,
    surface: ScrollbarSurface,
) -> Option<ScrollBar> {
    if !should_show(content_len, viewport_len) {
        return None;
    }

    let (bg, track_fg, thumb_fg) = match surface {
        ScrollbarSurface::Panel => (
            parse_color(&theme.panel_bg),
            parse_color(&theme.panel_border),
            parse_color(&theme.selection_bg),
        ),
        ScrollbarSurface::Popup => (
            parse_color(&theme.popup_bg),
            parse_color(&theme.popup_border),
            parse_color(&theme.selection_bg),
        ),
    };

    Some(
        ScrollBar::vertical(lengths(content_len, viewport_len))
            .offset(offset)
            .glyph_set(GlyphSet::unicode())
            .arrows(ScrollBarArrows::None)
            .track_click_behavior(TrackClickBehavior::JumpToClick)
            .track_style(Style::new().fg(track_fg).bg(bg))
            .thumb_style(Style::new().fg(thumb_fg).bg(bg)),
    )
}

/// Scroll position of a scrolled view.
#[derive(Debug, Clone, Copy)]
pub struct ScrollView {
    pub content_len: usize,
    pub viewport_len: usize,
    pub offset: usize,
}

/// Where a scrollbar is drawn and how mouse hits find it.
#[derive(Clone, Copy)]
pub struct ScrollTarget<'a> {
    pub surface: ScrollbarSurface,
    pub hits: Option<&'a ScrollbarUiState>,
    pub id: ScrollTargetId,
}

/// Render a vertical scrollbar into `track` when content overflows the viewport.
pub fn render_vertical(
    f: &mut Frame,
    track: Rect,
    view: ScrollView,
    theme: &Theme,
    target: ScrollTarget,
) {
    if track.width == 0 || track.height == 0 {
        return;
    }
    let Some(bar) = vertical_bar(
        view.content_len,
        view.viewport_len,
        view.offset,
        theme,
        target.surface,
    ) else {
        return;
    };
    if let Some(hits) = target.hits {
        hits.register(ScrollbarHitTarget {
            area: track,
            content_len: view.content_len,
            viewport_len: view.viewport_len,
            offset: view.offset,
            id: target.id,
        });
    }
    f.render_widget(&bar, track);
}

/// Render into the right column of `content_area`.
pub fn render_vertical_right(
    f: &mut Frame,
    content_area: Rect,
    view: ScrollView,
    theme: &Theme,
    target: ScrollTarget,
) {
    render_vertical(f, track_area_right(content_area), view, theme, target);
}

/// Render into the right column inside a bordered block covering `block_area`.
pub fn render_vertical_inside_block(
    f: &mut Frame,
    block_area: Rect,
    view: ScrollView,
    theme: &Theme,
    target: ScrollTarget,
) {
    render_vertical(f, track_area_inside_block(block_area), view, theme, target);
}
