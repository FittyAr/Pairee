//! What a plugin hands back as a widget (the return value of `peek`, the
//! argument of `pairee.preview_widget`, the children of a `ui.Layout`)
//! turned into a [`Renderable`].
//!
//! Accepted: the widget userdata (`Paragraph`, `List`, `Table`, `Gauge`,
//! `Layout`, the image of `pairee.image.show`), text (`Span`, `Line`,
//! `Text` or a string, shown as a paragraph) and the plain tables the 1.0
//! constructors returned (`{type = "Paragraph", text = ...}`, `Gauge`,
//! `List`, `Table`, `Span`, `Line`). Images only come from
//! `pairee.image.show`, which checked that the plugin may show the file;
//! no table form builds one. The result must stay within the
//! `Renderable` size limits.

use super::budget::{Budget, nested};
use super::geometry::{ClearWidget, PlacedWidget};
use super::text::{line_arg, span_arg, text_arg};
use crate::app::state::popup::plugin_widget::{
    PluginImage, Renderable, WBar, WFill, WGauge, WLayout, WLine, WList, WParagraph, WRow, WSpan,
    WStyle, WTable, WText,
};
use mlua::{Table, Value};

fn error(message: impl Into<String>) -> mlua::Error {
    mlua::Error::RuntimeError(message.into())
}

/// The widget held by a userdata, if it is one.
fn from_userdata(ud: &mlua::AnyUserData) -> mlua::Result<Renderable> {
    if let Ok(p) = ud.borrow::<WParagraph>() {
        return Ok(Renderable::Paragraph(p.clone()));
    }
    if let Ok(l) = ud.borrow::<WList>() {
        return Ok(Renderable::List(l.clone()));
    }
    if let Ok(t) = ud.borrow::<WTable>() {
        return Ok(Renderable::Table(t.clone()));
    }
    if let Ok(g) = ud.borrow::<WGauge>() {
        return Ok(Renderable::Gauge(g.clone()));
    }
    if let Ok(l) = ud.borrow::<WLayout>() {
        return Ok(Renderable::Layout(l.clone()));
    }
    if let Ok(i) = ud.borrow::<PluginImage>() {
        return Ok(Renderable::Image(i.clone()));
    }
    if let Ok(placed) = ud.borrow::<PlacedWidget>() {
        return Ok(placed.0.clone());
    }
    if ud.borrow::<ClearWidget>().is_ok() {
        return Ok(Renderable::Clear);
    }
    if let Ok(bar) = ud.borrow::<WBar>() {
        return Ok(Renderable::Bar(bar.clone()));
    }
    if let Ok(fill) = ud.borrow::<WFill>() {
        return Ok(Renderable::Fill(fill.clone()));
    }
    let is_text = ud.borrow::<WText>().is_ok()
        || ud.borrow::<WLine>().is_ok()
        || ud.borrow::<WSpan>().is_ok();
    if is_text {
        return Ok(Renderable::text(text_arg(&Value::UserData(ud.clone()))?));
    }
    Err(error("expected a ui widget"))
}

/// A 1.0 widget table (`{type = "...", ...}`).
fn from_legacy_table(table: &Table) -> mlua::Result<Renderable> {
    let kind: Option<String> = table.get("type")?;
    let mut budget = Budget::default();
    let mut text = |s: String| -> mlua::Result<WText> {
        let text = WText::raw(&s);
        budget.text(&text)?;
        Ok(text)
    };
    let mut strings = |key: &str| -> mlua::Result<Vec<WText>> {
        let Some(list) = table.get::<Option<Table>>(key)? else {
            return Ok(Vec::new());
        };
        list.sequence_values::<String>().map(|s| text(s?)).collect()
    };
    match kind.as_deref() {
        Some("Paragraph") => Ok(Renderable::text(text_arg(&table.get("text")?)?)),
        Some("Gauge") => Ok(Renderable::Gauge(WGauge {
            ratio: super::widgets::clamp_ratio(table.get::<Option<f64>>("ratio")?.unwrap_or(0.0)),
            label: table.get::<Option<String>>("label")?.map(|l| WSpan {
                content: l,
                ..WSpan::default()
            }),
            block: None,
            style: WStyle::default(),
            gauge_style: WStyle::default(),
        })),
        Some("List") => Ok(Renderable::List(WList {
            items: strings("items")?,
            block: None,
            style: WStyle::default(),
        })),
        Some("Table") => {
            let row = |cells| WRow {
                cells,
                height: 1,
                style: WStyle::default(),
            };
            let header = row(strings("headers")?);
            let rows: Vec<Table> = table.get::<Option<_>>("rows")?.unwrap_or_default();
            let rows = rows
                .iter()
                .map(|cells| {
                    let cells = cells.sequence_values::<String>().map(|c| text(c?));
                    Ok(row(cells.collect::<mlua::Result<_>>()?))
                })
                .collect::<mlua::Result<_>>()?;
            Ok(Renderable::Table(WTable {
                header: Some(header),
                rows,
                widths: Vec::new(),
                spacing: 1,
                block: None,
                style: WStyle::default(),
            }))
        }
        Some("Span") => Ok(Renderable::text(WText::lines(vec![WLine {
            spans: vec![legacy_span(&Value::Table(table.clone()))?],
            ..WLine::default()
        }]))),
        Some("Line") => {
            let spans: Vec<Value> = table.get::<Option<_>>("spans")?.unwrap_or_default();
            let spans = spans
                .iter()
                .map(legacy_span)
                .collect::<mlua::Result<Vec<_>>>()?;
            Ok(Renderable::text(WText::lines(vec![WLine {
                spans,
                ..WLine::default()
            }])))
        }
        _ => Err(error(
            "expected a ui widget (a table without a known `type`)",
        )),
    }
}

/// `{type = "Span", text = ..., style = "<color>"}`, or any span value
/// (a string or a `ui.Span`).
fn legacy_span(value: &Value) -> mlua::Result<WSpan> {
    let Value::Table(table) = value else {
        return span_arg(value);
    };
    let content: String = table.get::<Option<String>>("text")?.unwrap_or_default();
    let color: Option<String> = table.get("style")?;
    Ok(WSpan {
        content,
        style: WStyle {
            fg: color.filter(|c| !c.is_empty()),
            ..WStyle::default()
        },
        syntax: None,
    })
}

/// `value` as a widget; `nil` is an error.
pub fn renderable_arg(value: &Value) -> mlua::Result<Renderable> {
    renderable_at(value, 0)
}

/// Every item of `list` as a widget (`Layout:children`).
pub fn renderables_arg(list: &Table) -> mlua::Result<Vec<Renderable>> {
    renderables_at(list, 0)
}

fn renderables_at(list: &Table, depth: usize) -> mlua::Result<Vec<Renderable>> {
    let depth = nested(depth)?;
    let mut budget = Budget::default();
    list.sequence_values::<Value>()
        .map(|item| {
            let widget = renderable_at(&item?, depth)?;
            budget.widget(&widget)?;
            Ok(widget)
        })
        .collect()
}

/// [`renderable_arg`] for a value `depth` lists deep.
fn renderable_at(value: &Value, depth: usize) -> mlua::Result<Renderable> {
    let widget = match value {
        Value::UserData(ud) => from_userdata(ud)?,
        Value::Table(table) if table.contains_key("type")? => from_legacy_table(table)?,
        Value::Table(table) if is_widget_list(table)? => {
            Renderable::Stack(renderables_at(table, depth)?)
        }
        Value::Table(_) => Renderable::text(WText::lines(vec![line_arg(value)?])),
        Value::String(_) => Renderable::text(text_arg(value)?),
        other => {
            return Err(error(format!(
                "expected a ui widget, got {}",
                other.type_name()
            )));
        }
    };
    if !widget.within_limits() {
        return Err(error("ui widget too large to show"));
    }
    Ok(widget)
}

/// A list whose first item is a widget (not text): widgets drawn one over
/// the other. A list of spans stays a line.
fn is_widget_list(table: &Table) -> mlua::Result<bool> {
    Ok(match table.get::<Value>(1)? {
        Value::UserData(ud) => {
            ud.borrow::<WText>().is_err()
                && ud.borrow::<WLine>().is_err()
                && ud.borrow::<WSpan>().is_err()
        }
        Value::Table(t) => t.contains_key("type")? && !is_text_type(&t)?,
        _ => false,
    })
}

/// A 1.0 `{type = "Span" | "Line"}` table (text, not a widget).
fn is_text_type(table: &Table) -> mlua::Result<bool> {
    let kind: Option<String> = table.get("type")?;
    Ok(matches!(kind.as_deref(), Some("Span" | "Line")))
}

/// What `peek` returned: `nil` keeps the built-in preview.
pub fn to_renderable(value: &Value) -> mlua::Result<Option<Renderable>> {
    if value.is_nil() {
        return Ok(None);
    }
    renderable_arg(value).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::popup::plugin_widget::WTextBody;
    use mlua::Lua;

    fn lua() -> Lua {
        let lua = Lua::new();
        lua.globals()
            .set("ui", super::super::bind(&lua).unwrap())
            .unwrap();
        lua
    }

    fn convert(lua: &Lua, code: &str) -> mlua::Result<Option<Renderable>> {
        to_renderable(&lua.load(code).eval::<Value>()?)
    }

    #[test]
    fn userdata_text_and_legacy_tables_convert() {
        let lua = lua();
        assert!(convert(&lua, "return nil").unwrap().is_none());
        let para = convert(&lua, r#"return ui.Line("x"):bold()"#)
            .unwrap()
            .unwrap();
        assert!(matches!(para, Renderable::Paragraph(_)));
        let legacy = convert(&lua, r#"return { type = "Gauge", ratio = 2, label = "l" }"#)
            .unwrap()
            .unwrap();
        assert!(matches!(legacy, Renderable::Gauge(ref g) if g.ratio == 1.0));
        let table = convert(
            &lua,
            r#"return { type = "Table", headers = {"a"}, rows = {{"1"}, {"2"}} }"#,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(table, Renderable::Table(ref t) if t.rows.len() == 2));
        let line = convert(
            &lua,
            r#"return { type = "Line", spans = { { type = "Span", text = "a", style = "red" } } }"#,
        )
        .unwrap()
        .unwrap();
        let Renderable::Paragraph(p) = line else {
            panic!()
        };
        let WTextBody::Lines(lines) = p.text.body else {
            panic!()
        };
        assert_eq!(lines[0].spans[0].style.fg.as_deref(), Some("red"));
    }

    #[test]
    fn layouts_nest_and_bad_values_are_refused() {
        let lua = lua();
        let layout = convert(
            &lua,
            r#"return ui.Layout():direction(ui.Layout.HORIZONTAL)
                   :constraints({ ui.Constraint.Percentage(30), ui.Constraint.Fill() })
                   :children({ ui.Paragraph("a"), ui.List({"b"}) })"#,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(layout, Renderable::Layout(ref l) if l.children.len() == 2));
        assert!(convert(&lua, "return 42").is_err());
        assert!(convert(&lua, r#"return { type = "Image", path = "/etc/x.png" }"#).is_err());
        assert!(
            convert(
                &lua,
                "return ui.Paragraph(string.rep('x', 9 * 1024 * 1024))"
            )
            .is_err()
        );
    }

    #[test]
    fn hostile_values_fail_instead_of_exhausting_the_host() {
        let lua = lua();
        for code in [
            // Tables that contain themselves.
            "local t = {} t[1] = t return ui.Line(t)",
            "local t = { ui.Clear() } t[2] = t return t",
            // One table / userdata listed again and again.
            r#"local t = {"x"} for i = 1, 40 do t = {t, t} end return ui.Line(t)"#,
            "local s = ui.Text(string.rep('x', 1024 * 1024)) local a = {}              for i = 1, 64 do a[i] = s end return ui.List(a)",
            "local p = ui.Paragraph(string.rep('x', 1024 * 1024)) local a = {}              for i = 1, 64 do a[i] = p end return ui.Layout():children(a)",
            r#"local s = string.rep('x', 1024 * 1024) local a = {} for i = 1, 64 do a[i] = s end
               return { type = "List", items = a }"#,
            // A symbol drawn in every cell.
            "return ui.Fill(string.rep('x', 1024))",
            "return ui.Bar():symbol(string.rep('x', 1024))",
        ] {
            assert!(convert(&lua, code).is_err(), "{code}");
        }
    }

    #[test]
    fn legacy_values_convert_safely() {
        let lua = lua();
        let gauge = convert(&lua, r#"return { type = "Gauge", ratio = 0/0 }"#)
            .unwrap()
            .unwrap();
        assert!(matches!(gauge, Renderable::Gauge(ref g) if g.ratio == 0.0));
        let line = convert(
            &lua,
            r#"return { type = "Line", spans = { ui.Span("a"), "b", { text = "c" } } }"#,
        )
        .unwrap()
        .unwrap();
        let Renderable::Paragraph(p) = line else {
            panic!()
        };
        let WTextBody::Lines(lines) = p.text.body else {
            panic!()
        };
        assert_eq!(lines[0].spans.len(), 3);
    }
}
