//! Positions in a preview: `ui.Rect`, `ui.Pad`, `ui.Pos`, the position-only
//! widgets `ui.Bar`, `ui.Clear` and `ui.Fill`, `widget:area(rect)` and
//! `Layout:split(rect)`.
//!
//! A peek may return one widget or a list of widgets; each one can be
//! placed with `:area(rect)`, where `rect` is relative to the preview area
//! (`job.area`). The host clips what is drawn to the preview.

use super::convert::renderable_arg;
use super::style::add_style_methods;
use crate::app::state::popup::plugin_widget::geometry::split;
use crate::app::state::popup::plugin_widget::{
    Renderable, WBar, WFill, WLayout, WPad, WRect, WStyle, edge,
};
use mlua::{AnyUserData, Lua, MetaMethod, Table, UserData, UserDataFields, UserDataMethods, Value};

/// A widget placed in a rectangle (`widget:area(rect)`).
#[derive(Debug, Clone)]
pub struct PlacedWidget(pub Renderable);

impl UserData for PlacedWidget {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("area", place);
    }
}

/// `ui.Clear` (clears its area).
#[derive(Debug, Clone, Copy)]
pub struct ClearWidget;

impl UserData for ClearWidget {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("area", place);
    }
}

/// `widget:area(rect)` for any widget userdata.
fn place(_: &Lua, (ud, rect): (AnyUserData, AnyUserData)) -> mlua::Result<PlacedWidget> {
    let rect = *rect.borrow::<WRect>()?;
    let widget = match renderable_arg(&Value::UserData(ud))? {
        Renderable::Placed(_, inner) => *inner,
        other => other,
    };
    Ok(PlacedWidget(Renderable::Placed(rect, Box::new(widget))))
}

/// Adds `:area(rect)` to the widget types defined elsewhere.
pub fn add_area_method<T: 'static, M: UserDataMethods<T>>(methods: &mut M) {
    methods.add_function("area", place);
}

impl UserData for WRect {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, r| Ok(r.x));
        fields.add_field_method_get("y", |_, r| Ok(r.y));
        fields.add_field_method_get("w", |_, r| Ok(r.w));
        fields.add_field_method_get("h", |_, r| Ok(r.h));
        // 1.x preview jobs had `width` / `height`.
        fields.add_field_method_get("width", |_, r| Ok(r.w));
        fields.add_field_method_get("height", |_, r| Ok(r.h));
        fields.add_field_method_get("left", |_, r| Ok(r.x));
        fields.add_field_method_get("top", |_, r| Ok(r.y));
        fields.add_field_method_get("right", |_, r| Ok(r.right()));
        fields.add_field_method_get("bottom", |_, r| Ok(r.bottom()));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("pad", |_, r, pad: AnyUserData| {
            Ok(r.pad(*pad.borrow::<WPad>()?))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, r, other: AnyUserData| {
            Ok(other.borrow::<WRect>().is_ok_and(|o| *o == *r))
        });
        methods.add_meta_method(MetaMethod::ToString, |_, r, ()| {
            Ok(format!(
                "Rect{{x={}, y={}, w={}, h={}}}",
                r.x, r.y, r.w, r.h
            ))
        });
    }
}

impl UserData for WPad {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("top", |_, p| Ok(p.top));
        fields.add_field_method_get("right", |_, p| Ok(p.right));
        fields.add_field_method_get("bottom", |_, p| Ok(p.bottom));
        fields.add_field_method_get("left", |_, p| Ok(p.left));
    }
}

impl UserData for WBar {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        add_style_methods(methods);
        add_area_method(methods);
        methods.add_function("symbol", |_, (ud, symbol): (AnyUserData, String)| {
            ud.borrow_mut::<WBar>()?.symbol = Some(symbol);
            Ok(ud)
        });
    }
}

impl UserData for WFill {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        add_style_methods(methods);
        add_area_method(methods);
    }
}

/// `{x, y, w, h}` read from a table (missing fields are 0).
fn rect_from_table(t: &Table) -> mlua::Result<WRect> {
    let n = |k: &str| t.get::<Option<u16>>(k).map(Option::unwrap_or_default);
    Ok(WRect {
        x: n("x")?,
        y: n("y")?,
        w: n("w")?,
        h: n("h")?,
    })
}

/// `Layout:split(rect)`: one `Rect` per constraint, as the host draws it.
pub fn split_layout(layout: &WLayout, rect: WRect) -> mlua::Result<Vec<WRect>> {
    split(rect, layout.direction, layout.margin, &layout.constraints).ok_or_else(|| {
        mlua::Error::RuntimeError("Layout:split is only available inside Pairee's UI".into())
    })
}

/// Adds `Rect`, `Pad`, `Pos`, `Bar`, `Clear` and `Fill` to the `ui` table.
pub fn bind(lua: &Lua, ui: &Table) -> mlua::Result<()> {
    let rect = super::text::callable(lua, lua.create_function(|_, t: Table| rect_from_table(&t))?)?;
    rect.set("default", WRect::default())?;
    ui.set("Rect", rect)?;
    bind_pad(lua, ui)?;
    // `ui.Pos{"center", x =, y =, w =, h =}`: the `pos` of the dialogs.
    ui.set(
        "Pos",
        lua.create_function(|lua, t: Table| {
            let pos = lua.create_table()?;
            pos.set(
                1,
                t.get::<Option<String>>(1)?
                    .unwrap_or_else(|| "center".into()),
            )?;
            for key in ["x", "y", "w", "h"] {
                pos.set(key, t.get::<Option<i64>>(key)?.unwrap_or(0))?;
            }
            Ok(pos)
        })?,
    )?;
    ui.set(
        "Bar",
        lua.create_function(|_, edges: Option<u8>| {
            Ok(WBar {
                edges: edges.unwrap_or(edge::TOP) & edge::ALL,
                symbol: None,
                style: WStyle::default(),
            })
        })?,
    )?;
    ui.set(
        "Clear",
        lua.create_function(|lua, rect: Option<AnyUserData>| match rect {
            Some(rect) => Ok(Value::UserData(lua.create_userdata(PlacedWidget(
                Renderable::Placed(*rect.borrow::<WRect>()?, Box::new(Renderable::Clear)),
            ))?)),
            None => Ok(Value::UserData(lua.create_userdata(ClearWidget)?)),
        })?,
    )?;
    ui.set(
        "Fill",
        lua.create_function(|_, symbol: Option<String>| {
            Ok(WFill {
                symbol: symbol.unwrap_or_else(|| " ".into()),
                style: WStyle::default(),
            })
        })?,
    )
}

/// `ui.Pad(top, right, bottom, left)` and `Pad.top(n)`, ..., `Pad.x(n)`,
/// `Pad.y(n)`, `Pad.xy(x, y)`.
fn bind_pad(lua: &Lua, ui: &Table) -> mlua::Result<()> {
    let pad = super::text::callable(
        lua,
        lua.create_function(|_, (top, right, bottom, left): (u16, u16, u16, u16)| {
            Ok(WPad {
                top,
                right,
                bottom,
                left,
            })
        })?,
    )?;
    let one = |f: fn(u16) -> WPad| lua.create_function(move |_, n: u16| Ok(f(n)));
    pad.set(
        "top",
        one(|n| WPad {
            top: n,
            ..WPad::default()
        })?,
    )?;
    pad.set(
        "right",
        one(|n| WPad {
            right: n,
            ..WPad::default()
        })?,
    )?;
    pad.set(
        "bottom",
        one(|n| WPad {
            bottom: n,
            ..WPad::default()
        })?,
    )?;
    pad.set(
        "left",
        one(|n| WPad {
            left: n,
            ..WPad::default()
        })?,
    )?;
    pad.set(
        "x",
        one(|n| WPad {
            left: n,
            right: n,
            ..WPad::default()
        })?,
    )?;
    pad.set(
        "y",
        one(|n| WPad {
            top: n,
            bottom: n,
            ..WPad::default()
        })?,
    )?;
    pad.set(
        "xy",
        lua.create_function(|_, (x, y): (u16, u16)| {
            Ok(WPad {
                top: y,
                right: x,
                bottom: y,
                left: x,
            })
        })?,
    )?;
    ui.set("Pad", pad)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::popup::plugin_widget::WText;

    fn lua() -> Lua {
        let lua = Lua::new();
        lua.globals()
            .set("ui", super::super::bind(&lua).unwrap())
            .unwrap();
        lua
    }

    #[test]
    fn rects_and_pads_compute_in_lua() {
        let lua = lua();
        let (x, y, w, h, right, bottom, same): (u16, u16, u16, u16, u16, u16, bool) = lua
            .load(
                "local r = ui.Rect{ x = 1, y = 2, w = 10, h = 6 }:pad(ui.Pad.xy(2, 1))
                 return r.x, r.y, r.w, r.h, r.right, r.bottom,
                        r == ui.Rect{ x = 3, y = 3, w = 6, h = 4 }",
            )
            .eval()
            .unwrap();
        assert_eq!((x, y, w, h, right, bottom), (3, 3, 6, 4, 9, 7));
        assert!(same);
        let pos: Table = lua
            .load(r#"return ui.Pos{ "top-center", y = 2, w = 40, h = 3 }"#)
            .eval()
            .unwrap();
        assert_eq!(pos.get::<String>(1).unwrap(), "top-center");
        assert_eq!(pos.get::<u16>("w").unwrap(), 40);
    }

    #[test]
    fn widgets_are_placed_and_lists_stack() {
        let lua = lua();
        let value: Value = lua
            .load(
                r#"local area = ui.Rect{ w = 20, h = 5 }
                   return {
                     ui.Clear(area),
                     ui.Paragraph("hi"):area(ui.Rect{ x = 1, y = 1, w = 5, h = 1 }),
                     ui.Bar(ui.Edge.BOTTOM):symbol("-"):fg("red"),
                     ui.Fill("."):area(area),
                   }"#,
            )
            .eval()
            .unwrap();
        let Some(Renderable::Stack(children)) = super::super::to_renderable(&value).unwrap() else {
            panic!("a list of widgets is a stack")
        };
        assert_eq!(children.len(), 4);
        assert!(
            matches!(children[0], Renderable::Placed(_, ref c) if matches!(**c, Renderable::Clear))
        );
        let Renderable::Placed(rect, ref para) = children[1] else {
            panic!("{:?}", children[1])
        };
        assert_eq!((rect.x, rect.y, rect.w), (1, 1, 5));
        assert!(matches!(**para, Renderable::Paragraph(ref p) if p.text == WText::raw("hi")));
        assert!(matches!(children[2], Renderable::Bar(ref b) if b.edges == edge::BOTTOM));
        // A list of spans is still one line of text.
        let line: Value = lua
            .load(r#"return { ui.Span("a"), ui.Span("b") }"#)
            .eval()
            .unwrap();
        assert!(matches!(
            super::super::to_renderable(&line).unwrap(),
            Some(Renderable::Paragraph(_))
        ));
    }

    #[test]
    fn layout_split_needs_the_ui_solver() {
        crate::ui::quickview::install_layout_solver();
        let lua = lua();
        let (n, second_x, second_w): (usize, u16, u16) = lua
            .load(
                "local rects = ui.Layout():direction(ui.Layout.HORIZONTAL)
                     :constraints({ ui.Constraint.Length(4), ui.Constraint.Fill(1) })
                     :split(ui.Rect{ w = 10, h = 2 })
                 return #rects, rects[2].x, rects[2].w",
            )
            .eval()
            .unwrap();
        assert_eq!((n, second_x, second_w), (2, 4, 6));
    }
}
