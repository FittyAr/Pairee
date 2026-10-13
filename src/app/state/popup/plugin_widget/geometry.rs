//! Positions inside a plugin preview: `ui.Rect` / `ui.Pad` values, the
//! position-only widgets (`ui.Bar`, `ui.Clear`, `ui.Fill`) and splitting a
//! rectangle with a layout (`Layout:split(rect)`).
//!
//! Rectangles are relative to the preview area (`job.area` is `{x = 0,
//! y = 0, w, h}`); the UI layer offsets and clips them when it draws, so a
//! plugin can never paint outside its preview. Splitting needs the same
//! constraint solver the UI uses to draw layouts; the UI layer installs it
//! ([`install_layout_solver`]) so this core module stays free of ratatui.

use super::{WConstraint, WDirection, WStyle};
use std::sync::OnceLock;

/// A rectangle in cells, relative to the preview area.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WRect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl WRect {
    pub fn right(self) -> u16 {
        self.x.saturating_add(self.w)
    }

    pub fn bottom(self) -> u16 {
        self.y.saturating_add(self.h)
    }

    /// The rectangle shrunk by `pad` on each side (never negative).
    pub fn pad(self, pad: WPad) -> Self {
        let w = self.w.saturating_sub(pad.left.saturating_add(pad.right));
        let h = self.h.saturating_sub(pad.top.saturating_add(pad.bottom));
        Self {
            x: self.x.saturating_add(pad.left.min(self.w)),
            y: self.y.saturating_add(pad.top.min(self.h)),
            w,
            h,
        }
    }
}

/// Space to leave inside a rectangle (`ui.Pad`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WPad {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

/// A line along some edges of its area (`ui.Bar`): `edges` are
/// [`super::edge`] bits, `symbol` overrides the line character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WBar {
    pub edges: u8,
    pub symbol: Option<String>,
    pub style: WStyle,
}

/// Its whole area painted with `symbol` (default a space) in `style`
/// (`ui.Fill`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WFill {
    pub symbol: String,
    pub style: WStyle,
}

/// Splits `area` (after `margin`) into one rectangle per constraint.
pub type SplitFn = fn(WRect, WDirection, u16, &[WConstraint]) -> Vec<WRect>;

static SOLVER: OnceLock<SplitFn> = OnceLock::new();

/// Installs the solver the UI layer draws layouts with (first call wins).
pub fn install_layout_solver(solver: SplitFn) {
    let _ = SOLVER.set(solver);
}

/// `area` split like the UI would draw a layout of `constraints`; `None`
/// when no UI installed its solver (e.g. a command-line subcommand).
pub fn split(
    area: WRect,
    direction: WDirection,
    margin: u16,
    constraints: &[WConstraint],
) -> Option<Vec<WRect>> {
    SOLVER
        .get()
        .map(|solve| solve(area, direction, margin, constraints))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_shrinks_without_underflow() {
        let rect = WRect {
            x: 2,
            y: 1,
            w: 10,
            h: 4,
        };
        let pad = WPad {
            top: 1,
            right: 2,
            bottom: 1,
            left: 3,
        };
        assert_eq!(
            rect.pad(pad),
            WRect {
                x: 5,
                y: 2,
                w: 5,
                h: 2
            }
        );
        let huge = WPad {
            top: 9,
            right: 9,
            bottom: 9,
            left: 9,
        };
        assert_eq!(rect.pad(huge).w, 0);
        assert_eq!(rect.pad(huge).h, 0);
        assert_eq!((rect.right(), rect.bottom()), (12, 5));
    }
}
