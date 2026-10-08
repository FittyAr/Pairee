use super::layout::vertical_bar_for_input;
use super::types::{ScrollTargetId, ScrollbarHitTarget};
use crossterm::event::{MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};
use tui_scrollbar::{ScrollBarInteraction, ScrollCommand};

/// Rows scrolled per mouse-wheel step over a scrolled view.
const WHEEL_STEP: usize = 3;

/// Apply a mouse event against last-frame hit targets.
/// Returns `Some(id, new_offset)` when a scrollbar consumed the event.
///
/// The wheel scrolls the view under the pointer; elsewhere it scrolls the
/// topmost view (the bar itself ignores where the wheel turned).
pub fn handle_mouse_on_targets(
    targets: &[ScrollbarHitTarget],
    mouse: MouseEvent,
    interaction: &mut ScrollBarInteraction,
) -> Option<(ScrollTargetId, usize)> {
    // Last registered = topmost overlay (popups after panels).
    let scrollable = targets.iter().rev().filter_map(|target| {
        vertical_bar_for_input(target.content_len, target.viewport_len, target.offset)
            .map(|bar| (target, bar, max_offset(target)))
    });
    for (target, _, max) in scrollable.clone() {
        if let Some(next) = wheel_offset(target.wheel_area, target.offset, mouse) {
            return Some((target.id, next.min(max)));
        }
    }
    for (target, bar, max) in scrollable {
        if let Some(ScrollCommand::SetOffset(next)) =
            bar.handle_mouse_event(target.area, mouse, interaction)
        {
            return Some((target.id, next.min(max)));
        }
    }
    None
}

fn max_offset(target: &ScrollbarHitTarget) -> usize {
    target
        .content_len
        .saturating_sub(target.viewport_len.max(1))
}

/// New offset for a wheel step over `wheel_area`, `None` for other events.
fn wheel_offset(wheel_area: Rect, offset: usize, mouse: MouseEvent) -> Option<usize> {
    if !wheel_area.contains(Position::new(mouse.column, mouse.row)) {
        return None;
    }
    match mouse.kind {
        MouseEventKind::ScrollUp => Some(offset.saturating_sub(WHEEL_STEP)),
        MouseEventKind::ScrollDown => Some(offset.saturating_add(WHEEL_STEP)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn target() -> ScrollbarHitTarget {
        ScrollbarHitTarget {
            area: Rect::new(19, 0, 1, 10),
            wheel_area: Rect::new(0, 0, 20, 10),
            content_len: 100,
            viewport_len: 10,
            offset: 5,
            id: ScrollTargetId::MultiRenamePreview,
        }
    }

    fn wheel(kind: MouseEventKind, column: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row: 3,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn wheel_over_content_scrolls_and_clamps() {
        let mut interaction = ScrollBarInteraction::default();
        let targets = [target()];
        let down = handle_mouse_on_targets(
            &targets,
            wheel(MouseEventKind::ScrollDown, 4),
            &mut interaction,
        );
        assert_eq!(down, Some((ScrollTargetId::MultiRenamePreview, 8)));
        let up = handle_mouse_on_targets(
            &targets,
            wheel(MouseEventKind::ScrollUp, 4),
            &mut interaction,
        );
        assert_eq!(up, Some((ScrollTargetId::MultiRenamePreview, 2)));
        let mut at_end = target();
        at_end.offset = 90;
        let clamped = handle_mouse_on_targets(
            &[at_end],
            wheel(MouseEventKind::ScrollDown, 4),
            &mut interaction,
        );
        assert_eq!(clamped, Some((ScrollTargetId::MultiRenamePreview, 90)));
    }

    #[test]
    fn wheel_scrolls_the_view_under_the_pointer_else_the_topmost() {
        let mut interaction = ScrollBarInteraction::default();
        let mut top = target();
        top.id = ScrollTargetId::HelpContent;
        top.wheel_area = Rect::new(40, 0, 20, 10);
        top.area = Rect::new(59, 0, 1, 10);
        let targets = [target(), top];
        let under = handle_mouse_on_targets(
            &targets,
            wheel(MouseEventKind::ScrollDown, 4),
            &mut interaction,
        );
        assert_eq!(
            under.map(|(id, _)| id),
            Some(ScrollTargetId::MultiRenamePreview)
        );
        let elsewhere = handle_mouse_on_targets(
            &targets,
            wheel(MouseEventKind::ScrollDown, 30),
            &mut interaction,
        );
        assert_eq!(
            elsewhere.map(|(id, _)| id),
            Some(ScrollTargetId::HelpContent)
        );
    }
}
