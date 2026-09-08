use ratatui::layout::Rect;
use tui_scrollbar::{ScrollBar, ScrollLengths};

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

pub(crate) fn lengths(content_len: usize, viewport_len: usize) -> ScrollLengths {
    ScrollLengths {
        content_len,
        viewport_len,
    }
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
            .arrows(tui_scrollbar::ScrollBarArrows::None)
            .track_click_behavior(tui_scrollbar::TrackClickBehavior::JumpToClick),
    )
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
