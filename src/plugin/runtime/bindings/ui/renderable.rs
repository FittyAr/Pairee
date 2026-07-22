//! M4-T4: `Renderable` enum — a unified handle that the renderer
//! can switch on to dispatch a widget to ratatui. Today the
//! pipeline still goes through `PluginWidget` (serde-shaped) for
//! back-compat with the existing `peek` return path; this enum
//! exists so M4-T2 widgets that don't roundtrip cleanly through
//! serde (e.g. layout-bearing widgets) can be sent through the
//! `preview_widget(opts, renderable)` overload without losing
//! fidelity.

use super::elements::cell::Cell;
use super::elements::gauge::Gauge;
use super::elements::line::Line;
use super::elements::list::List;
use super::elements::paragraph::Paragraph;
use super::elements::span::Span;
use super::elements::table::{Row, Table};
use super::elements::text::Text;
use super::geometry::Rect;

/// M4-T4: a single enum that captures every widget type the
/// renderer can handle. Plugins build a `Renderable` by passing
/// a userdata value to `pairee.preview_widget(opts, value)`; the
/// `Renderable` is then either sent over the `PluginRequest`
/// channel (where it becomes a `PluginWidget` for the existing
/// `quickview.rs` renderer) or held in the binding for direct
/// rendering (M4-T2 will add the direct path).
#[derive(Debug, Clone)]
pub enum Renderable {
    Span(Span),
    Line(Line),
    Text(Text),
    Paragraph(Paragraph),
    List(List),
    Gauge(Gauge),
    Table(Table),
    Cell(Cell),
    Row(Row),
    /// A widget anchored to a specific area (e.g. a positioned
    /// `Paragraph(area=Rect(0,0,80,24))`).
    At { rect: Rect, inner: Box<Renderable> },
}

impl Renderable {
    /// Convert a generic Lua value into a `Renderable` if it
    /// matches one of the known widget userdata types.
    pub fn from_lua_value(val: mlua::Value) -> mlua::Result<Self> {
        match val {
            mlua::Value::UserData(ud) => {
                if let Ok(s) = ud.borrow::<Span>() { return Ok(Self::Span(s.clone())); }
                if let Ok(l) = ud.borrow::<Line>() { return Ok(Self::Line(l.clone())); }
                if let Ok(t) = ud.borrow::<Text>() { return Ok(Self::Text(t.clone())); }
                if let Ok(p) = ud.borrow::<Paragraph>() { return Ok(Self::Paragraph(p.clone())); }
                if let Ok(l) = ud.borrow::<List>() { return Ok(Self::List(l.clone())); }
                if let Ok(g) = ud.borrow::<Gauge>() { return Ok(Self::Gauge(g.clone())); }
                if let Ok(t) = ud.borrow::<Table>() { return Ok(Self::Table(t.clone())); }
                if let Ok(c) = ud.borrow::<Cell>() { return Ok(Self::Cell(c.clone())); }
                if let Ok(r) = ud.borrow::<Row>() { return Ok(Self::Row(r.clone())); }
                Err(mlua::Error::RuntimeError(
                    "Renderable: unsupported value type".to_string(),
                ))
            }
            other => Err(mlua::Error::RuntimeError(format!(
                "Renderable: expected userdata, got {other:?}"
            ))),
        }
    }

    /// Boxed helper for the recursive `At` variant.
    pub fn at(self, rect: Rect) -> Self {
        Self::At { rect, inner: Box::new(self) }
    }
}
