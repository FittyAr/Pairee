//! Shared vertical scrollbar rendering and mouse hit-testing via [`tui_scrollbar`].
//!
//! Uses fractional thumbs (1/8-cell steps) and theme colors for track/thumb.
//! Apps own `content_len` / `viewport_len` / `offset`; this module draws and
//! registers hit targets so the input loop can drag/jump the thumb.

use crate::config::theme::Theme;
use crate::ui::theme_apply::parse_color;
use ratatui::{Frame, layout::Rect, style::Style};
use std::cell::RefCell;
use tui_scrollbar::{
    GlyphSet, ScrollBar, ScrollBarArrows, ScrollBarInteraction, ScrollCommand, ScrollLengths,
    TrackClickBehavior,
};

/// Which theme surface colors to use for track/thumb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarSurface {
    /// File panels and full-screen viewer/quickview.
    Panel,
    /// Popups, help, history, transfer, git.
    Popup,
}

/// Logical scroll region that can receive mouse drag/jump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollTargetId {
    HelpContent,
    About,
    UpdateNotes,
    Viewer,
    QuickView,
    PanelLeft,
    PanelRight,
    GitList,
    HistoryCommand,
    HistoryView,
    HistoryFolder,
    TransferJobs,
    TransferFiles,
    TransferLog,
    PluginSelect,
}

/// Geometry + identity for one scrollbar painted last frame.
#[derive(Debug, Clone)]
pub struct ScrollbarHitTarget {
    pub area: Rect,
    pub content_len: usize,
    pub viewport_len: usize,
    pub offset: usize,
    pub id: ScrollTargetId,
}

/// Persistent drag state + hit targets registered during the last paint.
#[derive(Debug, Default)]
pub struct ScrollbarUiState {
    pub interaction: ScrollBarInteraction,
    targets: RefCell<Vec<ScrollbarHitTarget>>,
}

impl ScrollbarUiState {
    pub fn clear_targets(&self) {
        self.targets.borrow_mut().clear();
    }

    pub fn register(&self, target: ScrollbarHitTarget) {
        self.targets.borrow_mut().push(target);
    }

    pub fn targets_snapshot(&self) -> Vec<ScrollbarHitTarget> {
        self.targets.borrow().clone()
    }
}

/// Rightmost 1-cell column of `area` for a vertical track.
pub fn track_area_right(area: Rect) -> Rect {
    if area.width == 0 || area.height == 0 {
        return Rect::default();
    }
    Rect {
        x: area.x + area.width.saturating_sub(1),
        y: area.y,
        width: 1,
        height: area.height,
    }
}

/// Track area inside a full-bordered block (1 cell inset on each side, then right column).
pub fn track_area_inside_block(block_area: Rect) -> Rect {
    if block_area.width < 3 || block_area.height < 3 {
        return Rect::default();
    }
    let inner = Rect {
        x: block_area.x.saturating_add(1),
        y: block_area.y.saturating_add(1),
        width: block_area.width.saturating_sub(2),
        height: block_area.height.saturating_sub(2),
    };
    track_area_right(inner)
}

/// Whether a scrollbar should be shown for the given scroll metrics.
pub fn should_show(content_len: usize, viewport_len: usize) -> bool {
    content_len > 0 && viewport_len > 0 && content_len > viewport_len
}

fn lengths(content_len: usize, viewport_len: usize) -> ScrollLengths {
    ScrollLengths {
        content_len,
        viewport_len,
    }
}

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

/// Unthemed bar for mouse hit-testing (geometry only).
pub fn vertical_bar_for_input(
    content_len: usize,
    viewport_len: usize,
    offset: usize,
) -> Option<ScrollBar> {
    if !should_show(content_len, viewport_len) {
        return None;
    }
    Some(
        ScrollBar::vertical(lengths(content_len, viewport_len))
            .offset(offset)
            .arrows(ScrollBarArrows::None)
            .track_click_behavior(TrackClickBehavior::JumpToClick),
    )
}

/// Apply a mouse event against last-frame hit targets.
/// Returns `Some(id, new_offset)` when a scrollbar consumed the event.
pub fn handle_mouse_on_targets(
    targets: &[ScrollbarHitTarget],
    mouse: crossterm::event::MouseEvent,
    interaction: &mut ScrollBarInteraction,
) -> Option<(ScrollTargetId, usize)> {
    // Last registered = topmost overlay (popups after panels).
    for target in targets.iter().rev() {
        let Some(bar) =
            vertical_bar_for_input(target.content_len, target.viewport_len, target.offset)
        else {
            continue;
        };
        if let Some(ScrollCommand::SetOffset(next)) =
            bar.handle_mouse_event(target.area, mouse, interaction)
        {
            let max = target
                .content_len
                .saturating_sub(target.viewport_len.max(1));
            return Some((target.id, next.min(max)));
        }
    }
    None
}

/// Render a vertical scrollbar into `track` when content overflows the viewport.
pub fn render_vertical(
    f: &mut Frame,
    track: Rect,
    content_len: usize,
    viewport_len: usize,
    offset: usize,
    theme: &Theme,
    surface: ScrollbarSurface,
    hits: Option<&ScrollbarUiState>,
    id: ScrollTargetId,
) {
    if track.width == 0 || track.height == 0 {
        return;
    }
    let Some(bar) = vertical_bar(content_len, viewport_len, offset, theme, surface) else {
        return;
    };
    if let Some(hits) = hits {
        hits.register(ScrollbarHitTarget {
            area: track,
            content_len,
            viewport_len,
            offset,
            id,
        });
    }
    f.render_widget(&bar, track);
}

/// Render into the right column of `content_area`.
pub fn render_vertical_right(
    f: &mut Frame,
    content_area: Rect,
    content_len: usize,
    viewport_len: usize,
    offset: usize,
    theme: &Theme,
    surface: ScrollbarSurface,
    hits: Option<&ScrollbarUiState>,
    id: ScrollTargetId,
) {
    render_vertical(
        f,
        track_area_right(content_area),
        content_len,
        viewport_len,
        offset,
        theme,
        surface,
        hits,
        id,
    );
}

/// Render into the right column inside a bordered block covering `block_area`.
pub fn render_vertical_inside_block(
    f: &mut Frame,
    block_area: Rect,
    content_len: usize,
    viewport_len: usize,
    offset: usize,
    theme: &Theme,
    surface: ScrollbarSurface,
    hits: Option<&ScrollbarUiState>,
    id: ScrollTargetId,
) {
    render_vertical(
        f,
        track_area_inside_block(block_area),
        content_len,
        viewport_len,
        offset,
        theme,
        surface,
        hits,
        id,
    );
}

/// First visible index for a list that keeps `cursor` roughly centered.
pub fn centered_scroll(cursor: usize, content_len: usize, viewport_len: usize) -> usize {
    if content_len <= viewport_len || viewport_len == 0 {
        return 0;
    }
    let max_scroll = content_len.saturating_sub(viewport_len);
    let ideal = cursor.saturating_sub(viewport_len / 2);
    ideal.min(max_scroll)
}

/// Keep `cursor` inside the visible window starting at `offset`.
pub fn clamp_cursor_to_offset(
    cursor: &mut usize,
    offset: usize,
    viewport_len: usize,
    content_len: usize,
) {
    if content_len == 0 {
        *cursor = 0;
        return;
    }
    let first = offset.min(content_len - 1);
    let last = offset
        .saturating_add(viewport_len.saturating_sub(1))
        .min(content_len - 1);
    if *cursor < first || *cursor > last {
        *cursor = first;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn track_area_right_uses_last_column() {
        let area = Rect::new(10, 5, 40, 20);
        let track = track_area_right(area);
        assert_eq!(track.x, 49);
        assert_eq!(track.y, 5);
        assert_eq!(track.width, 1);
        assert_eq!(track.height, 20);
    }

    #[test]
    fn track_area_right_empty_when_zero_size() {
        assert_eq!(track_area_right(Rect::new(0, 0, 0, 10)), Rect::default());
        assert_eq!(track_area_right(Rect::new(0, 0, 5, 0)), Rect::default());
    }

    #[test]
    fn track_area_inside_block_insets_then_right() {
        let block = Rect::new(0, 0, 20, 10);
        let track = track_area_inside_block(block);
        assert_eq!(track.x, 18); // width 20 → inner ends at 18
        assert_eq!(track.y, 1);
        assert_eq!(track.width, 1);
        assert_eq!(track.height, 8);
    }

    #[test]
    fn should_show_only_when_overflow() {
        assert!(!should_show(0, 10));
        assert!(!should_show(10, 0));
        assert!(!should_show(10, 10));
        assert!(!should_show(5, 10));
        assert!(should_show(11, 10));
    }

    #[test]
    fn vertical_bar_none_when_no_overflow() {
        let theme = Theme::default();
        assert!(vertical_bar(5, 10, 0, &theme, ScrollbarSurface::Popup).is_none());
        assert!(vertical_bar(20, 10, 3, &theme, ScrollbarSurface::Panel).is_some());
    }

    #[test]
    fn centered_scroll_clamps_to_range() {
        assert_eq!(centered_scroll(0, 100, 20), 0);
        assert_eq!(centered_scroll(50, 100, 20), 40);
        assert_eq!(centered_scroll(99, 100, 20), 80);
        assert_eq!(centered_scroll(5, 10, 20), 0);
    }

    #[test]
    fn clamp_cursor_to_offset_moves_out_of_view() {
        let mut cursor = 0;
        clamp_cursor_to_offset(&mut cursor, 10, 5, 50);
        assert_eq!(cursor, 10);
        cursor = 12;
        clamp_cursor_to_offset(&mut cursor, 10, 5, 50);
        assert_eq!(cursor, 12);
    }

    #[test]
    fn hit_state_registers_targets() {
        let state = ScrollbarUiState::default();
        state.clear_targets();
        state.register(ScrollbarHitTarget {
            area: Rect::new(0, 0, 1, 10),
            content_len: 100,
            viewport_len: 10,
            offset: 0,
            id: ScrollTargetId::Viewer,
        });
        assert_eq!(state.targets_snapshot().len(), 1);
        state.clear_targets();
        assert!(state.targets_snapshot().is_empty());
    }
}
