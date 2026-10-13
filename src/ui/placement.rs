//! Where a dialog sits: centered by its renderer, then moved (and, for
//! dialogs whose content flows, resized) by the user with the mouse.
//!
//! The renderers keep computing centered areas; [`place`] applies the open
//! dialog's [`Placement`] to every such area and keeps it on screen, so all
//! dialogs move without each one knowing about it.

use ratatui::layout::Rect;
use std::cell::Cell;

/// Smallest size a dialog can be shrunk to.
const MIN_SIZE: (u16, u16) = (20, 5);

/// Offset from the centered position and extra size, in cells.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Placement {
    pub offset: (i16, i16),
    pub grow: (i16, i16),
}

const NONE: Placement = Placement {
    offset: (0, 0),
    grow: (0, 0),
};

thread_local! {
    static CURRENT: Cell<Placement> = const { Cell::new(NONE) };
    static RESIZABLE: Cell<bool> = const { Cell::new(false) };
    /// What the first placed area really got once kept on screen.
    static EFFECTIVE: Cell<Option<Placement>> = const { Cell::new(None) };
}

/// Applies `placement` to the dialogs painted until [`finish`];
/// `resizable` dialogs show a resize handle.
pub fn apply(placement: Placement, resizable: bool) {
    CURRENT.with(|current| current.set(placement));
    RESIZABLE.with(|current| current.set(resizable));
    EFFECTIVE.with(|effective| effective.set(None));
}

/// Stops placing dialogs and returns the placement the dialog really got
/// (after being kept on screen), if one was placed.
pub fn finish() -> Option<Placement> {
    let effective = EFFECTIVE.with(Cell::take);
    apply(NONE, false);
    effective
}

/// Runs `paint` with dialogs centered where their renderers put them (for
/// the dialogs under the top one, which the user did not move), then puts
/// the top dialog's placement back.
pub fn unplaced<R>(paint: impl FnOnce() -> R) -> R {
    let saved = (
        CURRENT.with(Cell::get),
        RESIZABLE.with(Cell::get),
        EFFECTIVE.with(Cell::get),
    );
    CURRENT.with(|current| current.set(NONE));
    RESIZABLE.with(|current| current.set(false));
    let result = paint();
    CURRENT.with(|current| current.set(saved.0));
    RESIZABLE.with(|current| current.set(saved.1));
    EFFECTIVE.with(|effective| effective.set(saved.2));
    result
}

/// Whether the dialog being painted can be resized.
pub fn resizable() -> bool {
    RESIZABLE.with(Cell::get)
}

/// `area` (centered on `screen` by a renderer) moved and resized by the
/// current placement, always inside `screen`.
pub fn place(area: Rect, screen: Rect) -> Rect {
    let placement = CURRENT.with(Cell::get);
    let placed = placed(area, screen, placement);
    EFFECTIVE.with(|effective| {
        if effective.get().is_none() {
            effective.set(Some(Placement {
                offset: (
                    (i32::from(placed.x) - i32::from(area.x)
                        + (i32::from(placed.width) - i32::from(area.width)) / 2)
                        as i16,
                    (i32::from(placed.y) - i32::from(area.y)
                        + (i32::from(placed.height) - i32::from(area.height)) / 2)
                        as i16,
                ),
                grow: (
                    (i32::from(placed.width) - i32::from(area.width)) as i16,
                    (i32::from(placed.height) - i32::from(area.height)) as i16,
                ),
            }));
        }
    });
    placed
}

fn placed(area: Rect, screen: Rect, placement: Placement) -> Rect {
    if placement == NONE {
        return area;
    }
    let size = |len: u16, grow: i16, min: u16, max: u16| {
        let wanted = (i32::from(len) + i32::from(grow)).max(i32::from(min.min(len)));
        wanted.clamp(0, i32::from(max)) as u16
    };
    let width = size(area.width, placement.grow.0, MIN_SIZE.0, screen.width);
    let height = size(area.height, placement.grow.1, MIN_SIZE.1, screen.height);
    // Grow around the centre, then shift.
    let x = i32::from(area.x) - (i32::from(width) - i32::from(area.width)) / 2
        + i32::from(placement.offset.0);
    let y = i32::from(area.y) - (i32::from(height) - i32::from(area.height)) / 2
        + i32::from(placement.offset.1);
    let x = x.clamp(i32::from(screen.x), i32::from(screen.right() - width));
    let y = y.clamp(i32::from(screen.y), i32::from(screen.bottom() - height));
    Rect::new(x as u16, y as u16, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = Rect::new(0, 0, 100, 30);

    fn placed(placement: Placement, area: Rect) -> Rect {
        apply(placement, true);
        let out = place(area, SCREEN);
        finish();
        out
    }

    #[test]
    fn the_effective_placement_is_what_fit_on_screen() {
        let area = Rect::new(30, 10, 40, 10);
        apply(
            Placement {
                offset: (500, 0),
                grow: (0, 0),
            },
            false,
        );
        place(area, SCREEN);
        let effective = finish().expect("placed");
        assert_eq!(effective.offset, (30, 0));
        assert!(finish().is_none());
    }

    #[test]
    fn without_a_placement_areas_stay_put() {
        let area = Rect::new(30, 10, 40, 10);
        assert_eq!(place(area, SCREEN), area);
    }

    #[test]
    fn offsets_move_and_stay_on_screen() {
        let area = Rect::new(30, 10, 40, 10);
        let moved = Placement {
            offset: (5, -3),
            ..Placement::default()
        };
        assert_eq!(placed(moved, area), Rect::new(35, 7, 40, 10));
        let far = Placement {
            offset: (500, 500),
            ..Placement::default()
        };
        assert_eq!(placed(far, area), Rect::new(60, 20, 40, 10));
        let negative = Placement {
            offset: (-500, -500),
            ..Placement::default()
        };
        assert_eq!(placed(negative, area), Rect::new(0, 0, 40, 10));
    }

    #[test]
    fn growing_keeps_the_centre_and_limits_the_size() {
        let area = Rect::new(30, 10, 40, 10);
        let grown = Placement {
            grow: (10, 4),
            ..Placement::default()
        };
        assert_eq!(placed(grown, area), Rect::new(25, 8, 50, 14));
        let huge = Placement {
            grow: (500, 500),
            ..Placement::default()
        };
        assert_eq!(placed(huge, area), SCREEN);
        let shrunk = Placement {
            grow: (-500, -500),
            ..Placement::default()
        };
        assert_eq!(placed(shrunk, area).width, MIN_SIZE.0);
        assert_eq!(placed(shrunk, area).height, MIN_SIZE.1);
    }
}
