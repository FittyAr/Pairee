//! M4-T3: `ui.Rect`, `ui.Pad`, `ui.Pos`, `ui.Constraint`,
//! `ui.Layout`, `ui.Align`, `ui.Wrap`, `ui.Edge` — geometry
//! primitives for the widget surface.

use mlua::{MetaMethod, UserData, UserDataMethods};
use ratatui::layout::{Constraint as RatConstraint, Rect as RatRect};

/// `ui.Rect{x, y, w, h}` userdata. Wraps `ratatui::layout::Rect`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    pub fn from_ratatui(r: RatRect) -> Self {
        Self { x: r.x, y: r.y, w: r.width, h: r.height }
    }

    pub fn to_ratatui(self) -> RatRect {
        RatRect::new(self.x, self.y, self.w, self.h)
    }
}

impl UserData for Rect {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("x", |_lua, this, x: u16| { this.x = x; Ok(*this) });
        methods.add_method_mut("y", |_lua, this, y: u16| { this.y = y; Ok(*this) });
        methods.add_method_mut("w", |_lua, this, w: u16| { this.w = w; Ok(*this) });
        methods.add_method_mut("h", |_lua, this, h: u16| { this.h = h; Ok(*this) });
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("Rect({}x{}@{},{})", this.w, this.h, this.x, this.y))
        });
    }
}

/// `ui.Rect(x, y, w, h)` callable.
pub fn bind_rect(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let rect = lua.create_table()?;
    rect.set(
        "__call",
        lua.create_function(|lua_ctx, args: mlua::MultiValue| {
            let v: Vec<u16> = args
                .into_iter()
                .filter_map(|a| match a { mlua::Value::Integer(n) => Some(n as u16), _ => None })
                .collect();
            let r = match v.as_slice() {
                [x, y, w, h] => Rect { x: *x, y: *y, w: *w, h: *h },
                _ => Rect::default(),
            };
            lua_ctx.create_userdata(r).map(mlua::Value::UserData)
        })?,
    )?;
    let mt = lua.create_table()?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    mt.set("__call", rect.get::<_, mlua::Function>("__call")?)?;
    rect.set_metatable(Some(mt));
    parent.set("Rect", rect)
}

/// `ui.Constraint.Min(n)` / `Length(n)` / `Percentage(n)` / `Ratio(n, m)`.
/// Stored as a transparent wrapper.
#[derive(Debug, Clone)]
pub struct Constraint(pub RatConstraint);

impl Constraint {
    pub fn min(n: u16) -> Self { Self(RatConstraint::Min(n)) }
    pub fn max(n: u16) -> Self { Self(RatConstraint::Max(n)) }
    pub fn length(n: u16) -> Self { Self(RatConstraint::Length(n)) }
    pub fn percentage(n: u16) -> Self { Self(RatConstraint::Percentage(n)) }
    pub fn ratio(n: u32, m: u32) -> Self { Self(RatConstraint::Ratio(n, m)) }
    pub fn fill(n: u16) -> Self { Self(RatConstraint::Fill(n)) }
}

impl UserData for Constraint {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(_methods: &mut M) {}
}

/// Factory table for `ui.Constraint`. M4-T1 only ships the
/// factory methods that plugins need most commonly.
pub fn bind_constraint(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let c = lua.create_table()?;
    macro_rules! factory {
        ($name:ident, $method:ident) => {
            c.set(stringify!($name), lua.create_function(move |lua_ctx, n: u16| {
                lua_ctx.create_userdata(Constraint::$method(n)).map(mlua::Value::UserData)
            })?)?;
        };
    }
    factory!(Min, min);
    factory!(Max, max);
    factory!(Length, length);
    factory!(Percentage, percentage);
    factory!(Fill, fill);
    c.set("Ratio", lua.create_function(|lua_ctx, (n, m): (u32, u32)| {
        lua_ctx.create_userdata(Constraint::ratio(n, m)).map(mlua::Value::UserData)
    })?)?;
    let mt = lua.create_table()?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    c.set_metatable(Some(mt));
    parent.set("Constraint", c)
}

/// `ui.Pad` userdata. Right now a simple 4-side padding.
#[derive(Debug, Clone, Copy, Default)]
pub struct Pad {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl Pad {
    pub fn uniform(n: u16) -> Self { Self { top: n, right: n, bottom: n, left: n } }
    pub fn xy(x: u16, y: u16) -> Self { Self { top: y, right: x, bottom: y, left: x } }
}

impl UserData for Pad {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("top",    |_lua, this, n: u16| { this.top    = n; Ok(*this) });
        methods.add_method_mut("right",  |_lua, this, n: u16| { this.right  = n; Ok(*this) });
        methods.add_method_mut("bottom", |_lua, this, n: u16| { this.bottom = n; Ok(*this) });
        methods.add_method_mut("left",   |_lua, this, n: u16| { this.left   = n; Ok(*this) });
    }
}

/// `ui.Pad(top, right, bottom, left)` callable.
pub fn bind_pad(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let p = lua.create_table()?;
    p.set("__call", lua.create_function(|lua_ctx, args: mlua::MultiValue| {
        let v: Vec<u16> = args
            .into_iter()
            .filter_map(|a| match a { mlua::Value::Integer(n) => Some(n as u16), _ => None })
            .collect();
        let pad = match v.as_slice() {
            [t, r, b, l] => Pad { top: *t, right: *r, bottom: *b, left: *l },
            [n] => Pad::uniform(*n),
            _ => Pad::default(),
        };
        lua_ctx.create_userdata(pad).map(mlua::Value::UserData)
    })?)?;
    let mt = lua.create_table()?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    mt.set("__call", p.get::<_, mlua::Function>("__call")?)?;
    p.set_metatable(Some(mt));
    parent.set("Pad", p)
}

/// `ui.Pos` — placeholder for the M4-T3 (top-center, x, y, w, h)
/// semantic. M4-T1 ships only the constructor; the dispatch
/// logic lands in M4-T4 alongside the Layout/Position system.
#[derive(Debug, Clone, Copy, Default)]
pub struct Pos;

impl UserData for Pos {}

pub fn bind_pos(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let p = lua.create_table()?;
    p.set("__call", lua.create_function(|_lua_ctx, _: mlua::MultiValue| Ok(mlua::Value::Nil))?)?;
    let mt = lua.create_table()?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    p.set_metatable(Some(mt));
    parent.set("Pos", p)
}

/// `ui.Align.LEFT` / `CENTER` / `RIGHT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align { Left, Center, Right }

impl Align {
    pub fn to_ratatui(self) -> ratatui::layout::Alignment {
        use ratatui::layout::Alignment;
        match self {
            Align::Left => Alignment::Left,
            Align::Center => Alignment::Center,
            Align::Right => Alignment::Right,
        }
    }
}

impl UserData for Align {}

pub fn bind_align(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let a = lua.create_table()?;
    a.set("LEFT", Align::Left)?;
    a.set("CENTER", Align::Center)?;
    a.set("RIGHT", Align::Right)?;
    let mt = lua.create_table()?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    a.set_metatable(Some(mt));
    parent.set("Align", a)
}

/// `ui.Wrap.NO` / `YES` / `TRIM`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrap { No, Yes, Trim }

impl Wrap {
    pub fn to_ratatui(self) -> ratatui::widgets::Wrap {
        use ratatui::widgets::Wrap as W;
        match self {
            Wrap::No => W { trim: false },
            Wrap::Yes => W { trim: false },
            Wrap::Trim => W { trim: true },
        }
    }
}

impl UserData for Wrap {}

pub fn bind_wrap(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let w = lua.create_table()?;
    w.set("NO", Wrap::No)?;
    w.set("YES", Wrap::Yes)?;
    w.set("TRIM", Wrap::Trim)?;
    let mt = lua.create_table()?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    w.set_metatable(Some(mt));
    parent.set("Wrap", w)
}

/// `ui.Edge` — bitmask over {TOP, RIGHT, BOTTOM, LEFT, ALL, NONE}.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Edge(pub u8);

impl Edge {
    pub const TOP: u8 = 0b0001;
    pub const RIGHT: u8 = 0b0010;
    pub const BOTTOM: u8 = 0b0100;
    pub const LEFT: u8 = 0b1000;
    pub const ALL: u8 = 0b1111;
    pub const NONE: u8 = 0b0000;

    pub fn contains(self, flag: u8) -> bool { (self.0 & flag) == flag }
}

impl UserData for Edge {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("contains", |_lua, this, flag: u8| Ok(this.contains(flag)));
    }
}

pub fn bind_edge(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let e = lua.create_table()?;
    e.set("NONE", Edge(Edge::NONE))?;
    e.set("TOP", Edge(Edge::TOP))?;
    e.set("RIGHT", Edge(Edge::RIGHT))?;
    e.set("BOTTOM", Edge(Edge::BOTTOM))?;
    e.set("LEFT", Edge(Edge::LEFT))?;
    e.set("ALL", Edge(Edge::ALL))?;
    let mt = lua.create_table()?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    e.set_metatable(Some(mt));
    parent.set("Edge", e)
}

/// `ui.Layout():direction(H|V):margin(n):constraints({...}):split(rect)`.
/// Stores a `ratatui::layout::Layout` configuration.
#[derive(Debug, Clone)]
pub struct Layout {
    pub direction: LayoutDirection,
    pub margin: u16,
    pub constraints: Vec<RatConstraint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection { Horizontal, Vertical }

impl Layout {
    pub fn new() -> Self {
        Self {
            direction: LayoutDirection::Vertical,
            margin: 0,
            constraints: Vec::new(),
        }
    }
}

impl Default for Layout { fn default() -> Self { Self::new() } }

impl UserData for Layout {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("direction", |_lua, this, dir: String| {
            this.direction = match dir.as_str() {
                "horizontal" | "H" => LayoutDirection::Horizontal,
                _ => LayoutDirection::Vertical,
            };
            Ok(this.clone())
        });
        methods.add_method_mut("margin", |_lua, this, m: u16| {
            this.margin = m;
            Ok(this.clone())
        });
        methods.add_method_mut("constraints", |_lua, this, cs: mlua::Table| {
            let mut out = Vec::new();
            for i in 1..=cs.len().unwrap_or(0) {
                if let Ok(mlua::Value::UserData(ud)) = cs.get::<_, mlua::Value>(i) {
                    if let Ok(c) = ud.borrow::<Constraint>() {
                        out.push(c.0);
                    }
                }
            }
            this.constraints = out;
            Ok(this.clone())
        });
    }
}

/// `ui.Layout()` callable.
pub fn bind_layout(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let l = lua.create_table()?;
    l.set("__call", lua.create_function(|lua_ctx, _: mlua::MultiValue| {
        lua_ctx.create_userdata(Layout::new()).map(mlua::Value::UserData)
    })?)?;
    let mt = lua.create_table()?;
    // Mirror `__call` on the metatable so Lua's method-lookup
    // chain (table → metatable) finds it.
    let inner_call = l.get::<_, mlua::Function>("__call")?;
    mt.set("__call", inner_call)?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    l.set_metatable(Some(mt));
    parent.set("Layout", l)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_rect_default() {
        let r = Rect::default();
        assert_eq!(r.x, 0);
        assert_eq!(r.w, 0);
    }

    #[test]
    fn test_rect_lua_callable() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        bind_rect(&lua, &table).unwrap();
        lua.globals().set("ui", table).unwrap();
        let v: mlua::Value = lua.load("return ui.Rect(1, 2, 3, 4)").eval().unwrap();
        let ud = v.as_userdata().unwrap();
        let r = ud.borrow::<Rect>().unwrap();
        assert_eq!((r.x, r.y, r.w, r.h), (1, 2, 3, 4));
    }

    #[test]
    fn test_constraint_factories() {
        let c1 = Constraint::min(1);
        let c2 = Constraint::percentage(50);
        assert!(matches!(c1.0, RatConstraint::Min(1)));
        assert!(matches!(c2.0, RatConstraint::Percentage(50)));
    }

    #[test]
    fn test_constraint_all_factories() {
        let _ = Constraint::min(1);
        let _ = Constraint::max(1);
        let _ = Constraint::length(42);
        let _ = Constraint::percentage(75);
        let _ = Constraint::fill(1);
        let _ = Constraint::ratio(1, 2);
    }

    #[test]
    fn test_pad_uniform_and_xy() {
        let p = Pad::uniform(3);
        assert_eq!(p.top, 3);
        assert_eq!(p.right, 3);
        assert_eq!(p.bottom, 3);
        assert_eq!(p.left, 3);
        let q = Pad::xy(1, 2);
        assert_eq!(q.top, 2);
        assert_eq!(q.right, 1);
        assert_eq!(q.bottom, 2);
        assert_eq!(q.left, 1);
    }

    #[test]
    fn test_pad_lua_callable_uniform() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        bind_pad(&lua, &table).unwrap();
        lua.globals().set("ui", table).unwrap();
        let v: mlua::Value = lua.load("return ui.Pad(2, 4, 6, 8)").eval().unwrap();
        let ud = v.as_userdata().unwrap();
        let p = ud.borrow::<Pad>().unwrap();
        assert_eq!(p.top, 2);
        assert_eq!(p.right, 4);
        assert_eq!(p.bottom, 6);
        assert_eq!(p.left, 8);
    }

    #[test]
    fn test_align_eq() {
        assert_eq!(Align::Left, Align::Left);
        assert_ne!(Align::Left, Align::Center);
        assert_ne!(Align::Left, Align::Right);
    }

    #[test]
    fn test_wrap_eq() {
        assert_eq!(Wrap::No, Wrap::No);
        assert_ne!(Wrap::No, Wrap::Trim);
    }

    #[test]
    fn test_edge_bitmask() {
        // Edge constants are typed `u8` directly.
        assert_eq!(Edge::ALL, 0b1111);
        assert_eq!(Edge::NONE, 0);
        assert_eq!(Edge::TOP, 0b0001);
        assert_eq!(Edge::RIGHT, 0b0010);
        assert_eq!(Edge::BOTTOM, 0b0100);
        assert_eq!(Edge::LEFT, 0b1000);
        // The bitmask covers TOP|RIGHT|BOTTOM|LEFT fully.
        assert_eq!(Edge::ALL, Edge::TOP | Edge::RIGHT | Edge::BOTTOM | Edge::LEFT);
        // The `contains` method works on an Edge userdata.
        let e = Edge(0b1111);
        assert!(e.contains(0b0001));
        assert!(e.contains(0b0010));
        assert!(e.contains(0b0100));
        assert!(e.contains(0b1000));
        assert!(!Edge(0).contains(0b0001));
    }

    #[test]
    fn test_layout_default_direction() {
        let l = Layout::new();
        assert_eq!(l.direction, LayoutDirection::Vertical);
        assert_eq!(l.margin, 0);
        assert!(l.constraints.is_empty());
    }

    #[test]
    fn test_layout_lua_callable() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        bind_layout(&lua, &table).unwrap();
        lua.globals().set("ui", table).unwrap();
        let v: mlua::Value = lua.load("return ui.Layout()").eval().unwrap();
        let ud = v.as_userdata().unwrap();
        let _l = ud.borrow::<Layout>().unwrap();
    }
}
