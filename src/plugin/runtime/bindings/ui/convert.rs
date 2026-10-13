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

use super::text::{line_arg, span_arg, text_arg};
use crate::app::state::popup::plugin_widget::{
    PluginImage, Renderable, WGauge, WLayout, WLine, WList, WParagraph, WRow, WSpan, WStyle,
    WTable, WText,
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
    let strings = |key: &str| -> mlua::Result<Vec<WText>> {
        Ok(table
            .get::<Option<Vec<String>>>(key)?
            .unwrap_or_default()
            .iter()
            .map(|s| WText::raw(s))
            .collect())
    };
    match kind.as_deref() {
        Some("Paragraph") => Ok(Renderable::text(text_arg(&table.get("text")?)?)),
        Some("Gauge") => Ok(Renderable::Gauge(WGauge {
            ratio: table
                .get::<Option<f64>>("ratio")?
                .unwrap_or(0.0)
                .clamp(0.0, 1.0),
            label: table.get::<Option<String>>("label")?.map(|l| WSpan {
                content: l,
                style: WStyle::default(),
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
            let rows: Vec<Vec<String>> = table.get::<Option<_>>("rows")?.unwrap_or_default();
            Ok(Renderable::Table(WTable {
                header: Some(WRow {
                    cells: strings("headers")?,
                    height: 1,
                    style: WStyle::default(),
                }),
                rows: rows
                    .iter()
                    .map(|cells| WRow {
                        cells: cells.iter().map(|c| WText::raw(c)).collect(),
                        height: 1,
                        style: WStyle::default(),
                    })
                    .collect(),
                widths: Vec::new(),
                spacing: 1,
                block: None,
                style: WStyle::default(),
            }))
        }
        Some("Span") => Ok(Renderable::text(WText::lines(vec![WLine {
            spans: vec![legacy_span(table)?],
            ..WLine::default()
        }]))),
        Some("Line") => {
            let spans: Vec<Table> = table.get::<Option<_>>("spans")?.unwrap_or_default();
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

/// `{type = "Span", text = ..., style = "<color>"}`, or any span value.
fn legacy_span(table: &Table) -> mlua::Result<WSpan> {
    let text: Option<String> = table.get("text")?;
    let Some(content) = text else {
        return span_arg(&Value::Table(table.clone()));
    };
    let color: Option<String> = table.get("style")?;
    Ok(WSpan {
        content,
        style: WStyle {
            fg: color.filter(|c| !c.is_empty()),
            ..WStyle::default()
        },
    })
}

/// `value` as a widget; `nil` is an error.
pub fn renderable_arg(value: &Value) -> mlua::Result<Renderable> {
    let widget = match value {
        Value::UserData(ud) => from_userdata(ud)?,
        Value::Table(table) if table.contains_key("type")? => from_legacy_table(table)?,
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
}
