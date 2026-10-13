use super::layout::{block_inner, lengths, should_show, track_area_right};
use super::types::{ScrollTargetId, ScrollbarHitTarget, ScrollbarSurface, ScrollbarUiState};
use crate::ui::resolved_theme::ResolvedTheme;
use ratatui::{Frame, layout::Rect, style::Style};
use tui_scrollbar::{GlyphSet, ScrollBar, ScrollBarArrows, TrackClickBehavior};

/// Build a themed vertical [`ScrollBar`], or `None` when content fits the viewport.
pub fn vertical_bar(
    content_len: usize,
    viewport_len: usize,
    offset: usize,
    theme: &ResolvedTheme,
    surface: ScrollbarSurface,
) -> Option<ScrollBar> {
    if !should_show(content_len, viewport_len) {
        return None;
    }

    let (bg, track_fg, thumb_fg) = match surface {
        ScrollbarSurface::Panel => (theme.panel_bg, theme.panel_border, theme.selection_bg),
        ScrollbarSurface::Popup => (theme.popup_bg, theme.popup_border, theme.selection_bg),
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
    theme: &ResolvedTheme,
    target: ScrollTarget,
) {
    render_track(f, track, track, view, theme, target);
}

/// Render into `track`; the mouse wheel over `wheel_area` scrolls too.
fn render_track(
    f: &mut Frame,
    track: Rect,
    wheel_area: Rect,
    view: ScrollView,
    theme: &ResolvedTheme,
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
            wheel_area,
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
    theme: &ResolvedTheme,
    target: ScrollTarget,
) {
    let track = track_area_right(content_area);
    render_track(f, track, content_area, view, theme, target);
}

/// Render into the right column inside a bordered block covering `block_area`.
pub fn render_vertical_inside_block(
    f: &mut Frame,
    block_area: Rect,
    view: ScrollView,
    theme: &ResolvedTheme,
    target: ScrollTarget,
) {
    let inner = block_inner(block_area);
    render_track(f, track_area_right(inner), inner, view, theme, target);
}

/// Records the wheel over `content` without drawing a bar, for views whose
/// scrollbar is turned off: the wheel scrolls them all the same.
pub fn register_wheel(content: Rect, view: ScrollView, target: ScrollTarget) {
    let Some(hits) = target.hits else {
        return;
    };
    if should_show(view.content_len, view.viewport_len) {
        hits.register(ScrollbarHitTarget {
            area: Rect::default(),
            wheel_area: content,
            content_len: view.content_len,
            viewport_len: view.viewport_len,
            offset: view.offset,
            id: target.id,
        });
    }
}
