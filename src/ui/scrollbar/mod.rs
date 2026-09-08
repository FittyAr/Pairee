//! Shared vertical scrollbar rendering and mouse hit-testing via [`tui_scrollbar`].
//!
//! Uses fractional thumbs (1/8-cell steps) and theme colors for track/thumb.
//! Apps own `content_len` / `viewport_len` / `offset`; this module draws and
//! registers hit targets so the input loop can drag/jump the thumb.

pub mod interaction;
pub mod layout;
pub mod render;
pub mod types;

pub use interaction::handle_mouse_on_targets;
pub use layout::{centered_scroll, clamp_cursor_to_offset};
pub use render::{render_vertical, render_vertical_inside_block, render_vertical_right};
pub use types::{ScrollTargetId, ScrollbarSurface, ScrollbarUiState};

#[cfg(test)]
mod tests {
    use super::layout::{should_show, track_area_inside_block, track_area_right};
    use super::render::vertical_bar;
    use super::types::ScrollbarHitTarget;
    use super::*;
    use crate::config::theme::Theme;
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
