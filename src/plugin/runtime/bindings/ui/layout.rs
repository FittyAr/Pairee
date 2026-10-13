//! `ui.Constraint` and `ui.Layout`.
//!
//! The layout is declarative: the plugin lists the children and their
//! constraints, and the host splits the preview area when it draws
//! (`ui.Layout():direction(ui.Layout.HORIZONTAL):constraints({...})
//! :children({...})`). `:split(rect)` returns the rectangles instead, split
//! by the solver the UI draws with, to place widgets with `:area(rect)`.

use super::convert::renderable_arg;
use crate::app::state::popup::plugin_widget::{WConstraint, WDirection, WLayout, WRect};
use mlua::{AnyUserData, Lua, Table, UserData, UserDataMethods, Value};

impl UserData for WConstraint {}

impl UserData for WLayout {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("direction", |_, (ud, dir): (AnyUserData, String)| {
            ud.borrow_mut::<WLayout>()?.direction = match dir.as_str() {
                "horizontal" => WDirection::Horizontal,
                "vertical" => WDirection::Vertical,
                other => {
                    return Err(mlua::Error::RuntimeError(format!(
                        "unknown direction {other:?} (use ui.Layout.HORIZONTAL / VERTICAL)"
                    )));
                }
            };
            Ok(ud)
        });
        methods.add_function("margin", |_, (ud, margin): (AnyUserData, u16)| {
            ud.borrow_mut::<WLayout>()?.margin = margin;
            Ok(ud)
        });
        methods.add_function("constraints", |_, (ud, list): (AnyUserData, Table)| {
            ud.borrow_mut::<WLayout>()?.constraints = constraint_list(&list)?;
            Ok(ud)
        });
        methods.add_method("split", |_, layout, rect: AnyUserData| {
            super::geometry::split_layout(layout, *rect.borrow::<WRect>()?)
        });
        super::geometry::add_area_method(methods);
        methods.add_function("children", |_, (ud, list): (AnyUserData, Table)| {
            let children = list
                .sequence_values::<Value>()
                .map(|child| renderable_arg(&child?))
                .collect::<mlua::Result<Vec<_>>>()?;
            ud.borrow_mut::<WLayout>()?.children = children;
            Ok(ud)
        });
    }
}

/// A list of `ui.Constraint` values.
pub fn constraint_list(list: &Table) -> mlua::Result<Vec<WConstraint>> {
    list.sequence_values::<AnyUserData>()
        .map(|c| Ok(*c?.borrow::<WConstraint>()?))
        .collect()
}

/// Adds `Constraint` and `Layout` to the `ui` table.
pub fn bind(lua: &Lua, ui: &Table) -> mlua::Result<()> {
    let constraint = lua.create_table()?;
    constraint.set(
        "Min",
        lua.create_function(|_, n: u16| Ok(WConstraint::Min(n)))?,
    )?;
    constraint.set(
        "Max",
        lua.create_function(|_, n: u16| Ok(WConstraint::Max(n)))?,
    )?;
    constraint.set(
        "Length",
        lua.create_function(|_, n: u16| Ok(WConstraint::Length(n)))?,
    )?;
    constraint.set(
        "Percentage",
        lua.create_function(|_, n: u16| Ok(WConstraint::Percentage(n.min(100))))?,
    )?;
    constraint.set(
        "Ratio",
        lua.create_function(|_, (a, b): (u32, u32)| Ok(WConstraint::Ratio(a, b.max(1))))?,
    )?;
    constraint.set(
        "Fill",
        lua.create_function(|_, n: Option<u16>| Ok(WConstraint::Fill(n.unwrap_or(1))))?,
    )?;
    ui.set("Constraint", constraint)?;

    let make = lua.create_function(|_, ()| {
        Ok(WLayout {
            direction: WDirection::Vertical,
            constraints: Vec::new(),
            margin: 0,
            children: Vec::new(),
        })
    })?;
    let layout = super::text::callable(lua, make)?;
    layout.set("HORIZONTAL", "horizontal")?;
    layout.set("VERTICAL", "vertical")?;
    ui.set("Layout", layout)
}
