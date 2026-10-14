//! `ui.Span`, `ui.Line` and `ui.Text`, and the conversions every widget
//! uses for its text arguments.
//!
//! * `ui.Span(text, style?)`: styled text without line breaks (`style` may
//!   be a `ui.Style` or a color string, as in the 1.0 `Span(text, color)`).
//! * `ui.Line(x)`: one line from a string, a `Span`, or a list of strings
//!   and spans; `:align(ui.Align.CENTER)`.
//! * `ui.Text(x)`: lines from a string (split at line breaks), a `Span`, a
//!   `Line` or a list of them; `ui.Text.parse(s)` keeps ANSI colors.

use super::budget::{Budget, nested};
use super::style::{add_style_methods, style_arg};
use crate::app::state::popup::plugin_widget::{WAlign, WLine, WSpan, WText, WTextBody};
use mlua::{AnyUserData, Lua, Table, UserData, UserDataMethods, Value};

fn type_error(what: &str, value: &Value) -> mlua::Error {
    mlua::Error::RuntimeError(format!("expected {what}, got {}", value.type_name()))
}

/// `"left"`, `"center"` or `"right"` (`ui.Align.*`).
pub fn align_arg(name: &str) -> mlua::Result<WAlign> {
    match name {
        "left" => Ok(WAlign::Left),
        "center" => Ok(WAlign::Center),
        "right" => Ok(WAlign::Right),
        other => Err(mlua::Error::RuntimeError(format!(
            "unknown alignment {other:?} (use ui.Align)"
        ))),
    }
}

pub fn span_arg(value: &Value) -> mlua::Result<WSpan> {
    match value {
        Value::String(s) => Ok(WSpan {
            content: s.to_string_lossy(),
            ..WSpan::default()
        }),
        Value::Integer(_) | Value::Number(_) => Ok(WSpan {
            content: value.to_string()?,
            ..WSpan::default()
        }),
        Value::UserData(ud) => Ok(ud.borrow::<WSpan>()?.clone()),
        other => Err(type_error("a string or ui.Span", other)),
    }
}

pub fn line_arg(value: &Value) -> mlua::Result<WLine> {
    line_at(value, 0)
}

/// [`line_arg`] for a value `depth` lists deep.
fn line_at(value: &Value, depth: usize) -> mlua::Result<WLine> {
    match value {
        Value::UserData(ud) if ud.borrow::<WLine>().is_ok() => Ok(ud.borrow::<WLine>()?.clone()),
        Value::Table(items) => {
            let depth = nested(depth)?;
            let mut budget = Budget::default();
            let mut spans = Vec::new();
            for item in items.sequence_values::<Value>() {
                let line = line_at(&item?, depth)?;
                budget.line(&line)?;
                match line {
                    line if line.style == Default::default() => spans.extend(line.spans),
                    line => spans.extend(line.spans.into_iter().map(|mut s| {
                        s.style = line.style.patch(&s.style);
                        s
                    })),
                }
            }
            Ok(WLine {
                spans,
                ..WLine::default()
            })
        }
        other => Ok(WLine {
            spans: vec![span_arg(other)?],
            ..WLine::default()
        }),
    }
}

pub fn text_arg(value: &Value) -> mlua::Result<WText> {
    match value {
        Value::Nil => Ok(WText::default()),
        Value::String(s) => Ok(WText::raw(&s.to_string_lossy())),
        Value::UserData(ud) if ud.borrow::<WText>().is_ok() => Ok(ud.borrow::<WText>()?.clone()),
        Value::Table(items) => {
            let mut budget = Budget::default();
            let lines = items
                .sequence_values::<Value>()
                .map(|item| {
                    let line = line_arg(&item?)?;
                    budget.line(&line)?;
                    Ok(line)
                })
                .collect::<mlua::Result<Vec<_>>>()?;
            Ok(WText::lines(lines))
        }
        other => Ok(WText::lines(vec![line_arg(other)?])),
    }
}

impl UserData for WSpan {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        add_style_methods(methods);
    }
}

impl UserData for WLine {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        add_style_methods(methods);
        methods.add_function("align", |_, (ud, align): (AnyUserData, String)| {
            ud.borrow_mut::<WLine>()?.align = Some(align_arg(&align)?);
            Ok(ud)
        });
    }
}

impl UserData for WText {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        add_style_methods(methods);
        methods.add_function("align", |_, (ud, align): (AnyUserData, String)| {
            ud.borrow_mut::<WText>()?.align = Some(align_arg(&align)?);
            Ok(ud)
        });
    }
}

/// A table callable as `name(...)` through `call`, with extra entries.
pub fn callable(lua: &Lua, call: mlua::Function) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    let meta = lua.create_table()?;
    let forward = lua.create_function(move |_, args: mlua::MultiValue| {
        let mut args = args.into_iter();
        args.next(); // the table itself
        call.call::<Value>(mlua::MultiValue::from_iter(args))
    })?;
    meta.set("__call", forward)?;
    table.set_metatable(Some(meta))?;
    Ok(table)
}

/// Adds `Span`, `Line` and `Text` to the `ui` table.
pub fn bind(lua: &Lua, ui: &Table) -> mlua::Result<()> {
    ui.set(
        "Span",
        lua.create_function(|_, (text, style): (Value, Value)| {
            let mut span = span_arg(&text)?;
            span.style = span.style.patch(&style_arg(&style)?);
            Ok(span)
        })?,
    )?;
    ui.set(
        "Line",
        lua.create_function(|_, value: Value| line_arg(&value))?,
    )?;
    let text = callable(
        lua,
        lua.create_function(|_, value: Value| text_arg(&value))?,
    )?;
    text.set(
        "parse",
        lua.create_function(|_, ansi: mlua::LuaString| {
            Ok(WText {
                body: WTextBody::Ansi(ansi.to_string_lossy()),
                ..WText::default()
            })
        })?,
    )?;
    ui.set("Text", text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lua() -> Lua {
        let lua = Lua::new();
        let ui = lua.create_table().unwrap();
        bind(&lua, &ui).unwrap();
        lua.globals().set("ui", ui).unwrap();
        lua
    }

    fn text(lua: &Lua, code: &str) -> WText {
        text_arg(&lua.load(code).eval::<Value>().unwrap()).unwrap()
    }

    #[test]
    fn strings_spans_and_lines_build_text() {
        let lua = lua();
        let t = text(&lua, r#"return ui.Text("a\nb")"#);
        assert_eq!(t, WText::raw("a\nb"));
        let t = text(
            &lua,
            r#"return ui.Text({ ui.Line({ "x", ui.Span("y", "red") }):align("center"), "z" })"#,
        );
        let WTextBody::Lines(lines) = &t.body else {
            panic!("{t:?}")
        };
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].spans.len(), 2);
        assert_eq!(lines[0].spans[1].style.fg.as_deref(), Some("red"));
        assert_eq!(lines[0].align, Some(WAlign::Center));
    }

    #[test]
    fn ansi_text_is_kept_for_the_ui() {
        let lua = lua();
        let t = text(&lua, r#"return ui.Text.parse("\27[31mred\27[0m")"#);
        assert!(matches!(t.body, WTextBody::Ansi(ref s) if s.contains("red")));
        assert!(lua.load(r#"return ui.Line(true)"#).exec().is_err());
        assert!(
            lua.load(r#"return ui.Line("x"):align("middle")"#)
                .exec()
                .is_err()
        );
    }
}
