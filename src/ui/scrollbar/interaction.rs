use super::layout::vertical_bar_for_input;
use super::types::{ScrollTargetId, ScrollbarHitTarget};
use tui_scrollbar::{ScrollBarInteraction, ScrollCommand};

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
