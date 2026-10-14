//! Positioned plugin widgets: `widget:area(rect)`, lists of widgets, and
//! the position-only widgets `ui.Bar`, `ui.Clear` and `ui.Fill`; plus the
//! layout solver `Layout:split(rect)` uses (the same split the drawing
//! does).

use super::convert;
use crate::app::state::popup::plugin_widget::geometry::install_layout_solver;
use crate::app::state::popup::plugin_widget::{WBar, WConstraint, WDirection, WFill, WRect, edge};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    widgets::Clear,
};

/// `rect` (relative to `area`) on screen, clipped to `area`.
pub fn placed(area: Rect, rect: WRect) -> Rect {
    let target = Rect {
        x: area.x.saturating_add(rect.x),
        y: area.y.saturating_add(rect.y),
        width: rect.w,
        height: rect.h,
    };
    target.intersection(area)
}

pub fn draw_clear(f: &mut Frame, area: Rect) {
    f.render_widget(Clear, area);
}

/// The fill symbol in every cell of `area`.
pub fn draw_fill(f: &mut Frame, area: Rect, fill: &WFill) {
    let style = convert::style(&fill.style);
    let symbol = if fill.symbol.is_empty() {
        " "
    } else {
        fill.symbol.as_str()
    };
    let buf = f.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(symbol).set_style(style);
            }
        }
    }
}

/// A line along each edge of the bar; the default symbol is a light box
/// line (horizontal or vertical by edge).
pub fn draw_bar(f: &mut Frame, area: Rect, bar: &WBar) {
    if area.is_empty() {
        return;
    }
    let style = convert::style(&bar.style);
    let symbol = |horizontal: bool| {
        bar.symbol
            .clone()
            .unwrap_or_else(|| if horizontal { "─" } else { "│" }.to_string())
    };
    let buf = f.buffer_mut();
    let (last_x, last_y) = (area.right() - 1, area.bottom() - 1);
    let mut paint = |x: u16, y: u16, s: &str| {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_symbol(s).set_style(style);
        }
    };
    for (bit, horizontal, fixed) in [
        (edge::TOP, true, area.y),
        (edge::BOTTOM, true, last_y),
        (edge::LEFT, false, area.x),
        (edge::RIGHT, false, last_x),
    ] {
        if bar.edges & bit == 0 {
            continue;
        }
        let s = symbol(horizontal);
        if horizontal {
            (area.left()..area.right()).for_each(|x| paint(x, fixed, &s));
        } else {
            (area.top()..area.bottom()).for_each(|y| paint(fixed, y, &s));
        }
    }
}

/// `area` split into one area per child (missing constraints share the
/// rest), as layouts are drawn.
pub fn split_areas(
    area: Rect,
    direction: WDirection,
    margin: u16,
    constraints: &[WConstraint],
    count: usize,
) -> Vec<Rect> {
    let area = area.inner(Margin::new(margin, margin));
    let constraints: Vec<Constraint> = (0..count)
        .map(|i| {
            constraints
                .get(i)
                .map_or(Constraint::Fill(1), convert::constraint)
        })
        .collect();
    let direction = match direction {
        WDirection::Horizontal => Direction::Horizontal,
        WDirection::Vertical => Direction::Vertical,
    };
    Layout::default()
        .direction(direction)
        .constraints(constraints)
        .split(area)
        .to_vec()
}

/// The solver of `Layout:split(rect)`: rectangles relative to the same
/// origin as `rect`.
fn solve(
    rect: WRect,
    direction: WDirection,
    margin: u16,
    constraints: &[WConstraint],
) -> Vec<WRect> {
    let area = Rect::new(rect.x, rect.y, rect.w, rect.h);
    split_areas(area, direction, margin, constraints, constraints.len())
        .into_iter()
        .map(|r| WRect {
            x: r.x,
            y: r.y,
            w: r.width,
            h: r.height,
        })
        .collect()
}

/// Lets `Layout:split(rect)` use the solver the quick view draws with.
pub fn install() {
    install_layout_solver(solve);
}
