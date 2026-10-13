//! Moving dialogs by their top border and resizing them by their corner.

use crate::app::state::{DialogStack, PopupType};
use crate::ui::placement::Placement;
use std::cell::Cell;
use std::mem::Discriminant;

/// Which dialog a placement belongs to: stack depth and dialog kind. A new
/// dialog (or the same kind opened again elsewhere in the stack) starts
/// centered.
type DialogKey = (usize, Discriminant<PopupType>);

fn key_of(dialogs: &DialogStack) -> Option<DialogKey> {
    dialogs
        .top()
        .map(|popup| (dialogs.depth(), std::mem::discriminant(popup)))
}

/// Where the user put the open dialog.
#[derive(Debug, Default)]
pub struct DialogPlacement {
    key: Option<DialogKey>,
    placement: Placement,
    /// What the last frame could honour (kept on screen), so a drag past
    /// the edge resumes as soon as the pointer comes back.
    effective: Cell<Option<Placement>>,
    drag: Option<Drag>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragKind {
    Move,
    Resize,
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    kind: DragKind,
    /// Pointer cell of the previous event (moves follow it step by step).
    last: (u16, u16),
    /// Pointer cell and placement when the drag began (a resize follows the
    /// total movement: the centre shifts by half of it).
    grab: (u16, u16),
    start: Placement,
}

impl DialogPlacement {
    /// Placement of the top dialog, and whether it may be resized.
    pub fn for_top(&self, dialogs: &DialogStack) -> (Placement, bool) {
        let resizable = dialogs
            .top()
            .is_some_and(|popup| popup.dialog().resizable());
        let placement = if self.key.is_some() && self.key == key_of(dialogs) {
            self.placement
        } else {
            Placement::default()
        };
        let placement = if resizable {
            placement
        } else {
            Placement {
                grow: (0, 0),
                ..placement
            }
        };
        (placement, resizable)
    }

    /// Records what the frame could honour.
    pub fn placed(&self, effective: Option<Placement>) {
        self.effective.set(effective);
    }

    /// Starts moving or resizing the top dialog from pointer cell `at`.
    pub fn start(&mut self, dialogs: &DialogStack, kind: DragKind, at: (u16, u16)) {
        let (current, _) = self.for_top(dialogs);
        self.key = key_of(dialogs);
        self.placement = self.effective.get().unwrap_or(current);
        self.drag = Some(Drag {
            kind,
            last: at,
            grab: at,
            start: self.placement,
        });
    }

    /// Follows the pointer to `at`. Returns whether a drag is going on.
    pub fn follow(&mut self, at: (u16, u16)) -> bool {
        let Some(drag) = self.drag else {
            return false;
        };
        let delta = |from: (u16, u16)| {
            (
                i32::from(at.0) - i32::from(from.0),
                i32::from(at.1) - i32::from(from.1),
            )
        };
        let add = |value: i16, delta: i32| (i32::from(value) + delta).clamp(-999, 999) as i16;
        self.placement = match drag.kind {
            DragKind::Move => {
                let base = self.effective.get().unwrap_or(self.placement);
                let (dx, dy) = delta(drag.last);
                Placement {
                    offset: (add(base.offset.0, dx), add(base.offset.1, dy)),
                    ..base
                }
            }
            // The corner follows the pointer: the dialog grows around its
            // centre, so the centre shifts by half the movement.
            DragKind::Resize => {
                let (dx, dy) = delta(drag.grab);
                let start = drag.start;
                Placement {
                    grow: (add(start.grow.0, dx), add(start.grow.1, dy)),
                    offset: (add(start.offset.0, dx / 2), add(start.offset.1, dy / 2)),
                }
            }
        };
        self.effective.set(None);
        self.drag = Some(Drag { last: at, ..drag });
        true
    }

    /// Ends a drag. Returns whether one was going on.
    pub fn stop(&mut self) -> bool {
        self.drag.take().is_some()
    }

    /// Puts the top dialog back in the centre at its own size.
    pub fn recenter(&mut self, dialogs: &DialogStack) {
        self.key = key_of(dialogs);
        self.placement = Placement::default();
        self.effective.set(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::popup::About;
    use crate::app::state::popup::ConfirmQuit;

    fn dialogs(popup: PopupType) -> DialogStack {
        let mut stack = DialogStack::new();
        stack.replace(popup);
        stack
    }

    #[test]
    fn a_placement_belongs_to_the_dialog_it_was_made_for() {
        let about = dialogs(PopupType::About(About { scroll_y: 0 }));
        let mut placement = DialogPlacement::default();
        placement.start(&about, DragKind::Move, (10, 10));
        assert!(placement.follow((15, 12)));
        assert!(placement.stop());
        assert_eq!(placement.for_top(&about).0.offset, (5, 2));
        let other = dialogs(PopupType::ConfirmQuit(ConfirmQuit));
        assert_eq!(placement.for_top(&other).0, Placement::default());
    }

    #[test]
    fn dragging_resumes_from_what_fit_on_screen() {
        let about = dialogs(PopupType::About(About { scroll_y: 0 }));
        let mut placement = DialogPlacement::default();
        placement.start(&about, DragKind::Move, (10, 10));
        placement.follow((90, 10));
        // The frame could only move it 30 columns.
        placement.placed(Some(Placement {
            offset: (30, 0),
            grow: (0, 0),
        }));
        placement.follow((89, 10));
        assert_eq!(placement.for_top(&about).0.offset, (29, 0));
    }

    #[test]
    fn only_resizable_dialogs_keep_their_size() {
        let quit = dialogs(PopupType::ConfirmQuit(ConfirmQuit));
        let mut placement = DialogPlacement::default();
        placement.start(&quit, DragKind::Resize, (10, 10));
        placement.follow((20, 14));
        let (got, resizable) = placement.for_top(&quit);
        assert!(!resizable);
        assert_eq!(got.grow, (0, 0));
    }
}
