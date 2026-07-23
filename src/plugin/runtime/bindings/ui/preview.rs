//! M4-T1: `pairee.preview_widget(opts, widget)` — push a Span /
//! Line / Text userdata into the preview pane. The dispatcher
//! converts the userdata to a `PluginWidget::RichSpan` /
//! `RichLine` / `RichText` and sends it through the existing
//! `PluginRequest::UpdatePluginWidget` channel.

use super::elements::line::Line;
use super::elements::span::Span;
use super::elements::text::Text;
use super::style::Style;
use crate::app::state::types::PluginWidget;
use crate::plugin::manager::PluginRequest;
use std::path::PathBuf;
use tokio::sync::mpsc::error::TrySendError;

/// Convert a `Span` userdata to a `PluginWidget::RichSpan`. If
/// `inherit_fg`/`inherit_bg`/`inherit_modifiers` are supplied, they
/// are merged in (the span's own settings take precedence over the
/// inherited ones, per CSS-like cascade).
fn span_to_plugin_with_inheritance(
    span: &Span,
    inherit_fg: Option<String>,
    inherit_bg: Option<String>,
    inherit_modifiers: ratatui::style::Modifier,
) -> PluginWidget {
    let (own_fg, own_bg) = extract_fg_bg(&span.style);
    let fg = own_fg.or(inherit_fg);
    let bg = own_bg.or(inherit_bg);
    let merged = span.style.inner.add_modifier | inherit_modifiers;
    PluginWidget::RichSpan {
        text: span.text.clone(),
        fg,
        bg,
        bold: merged.contains(ratatui::style::Modifier::BOLD),
        dim: merged.contains(ratatui::style::Modifier::DIM),
        italic: merged.contains(ratatui::style::Modifier::ITALIC),
        underline: merged.contains(ratatui::style::Modifier::UNDERLINED),
        blink: merged.contains(ratatui::style::Modifier::SLOW_BLINK),
        reverse: merged.contains(ratatui::style::Modifier::REVERSED),
        hidden: merged.contains(ratatui::style::Modifier::HIDDEN),
        crossed: merged.contains(ratatui::style::Modifier::CROSSED_OUT),
    }
}

fn span_to_plugin(span: &Span) -> PluginWidget {
    span_to_plugin_with_inheritance(span, None, None, ratatui::style::Modifier::empty())
}

fn line_to_plugin(line: &Line) -> PluginWidget {
    let (line_fg, line_bg) = extract_fg_bg(&line.style);
    let line_modifiers = line.style.inner.add_modifier;
    PluginWidget::RichLine {
        spans: line
            .spans
            .iter()
            .map(|s| {
                span_to_plugin_with_inheritance(
                    s,
                    line_fg.clone(),
                    line_bg.clone(),
                    line_modifiers,
                )
            })
            .collect(),
        fg: line_fg,
        bg: line_bg,
        bold: line_modifiers.contains(ratatui::style::Modifier::BOLD),
        dim: line_modifiers.contains(ratatui::style::Modifier::DIM),
        italic: line_modifiers.contains(ratatui::style::Modifier::ITALIC),
        underline: line_modifiers.contains(ratatui::style::Modifier::UNDERLINED),
    }
}

fn text_to_plugin(text: &Text) -> PluginWidget {
    let (fg, bg) = extract_fg_bg(&text.style);
    PluginWidget::RichText {
        lines: text.lines.iter().map(line_to_plugin).collect(),
        fg,
        bg,
        bold: text.style.inner.add_modifier.contains(ratatui::style::Modifier::BOLD),
        dim: text.style.inner.add_modifier.contains(ratatui::style::Modifier::DIM),
        italic: text
            .style
            .inner
            .add_modifier
            .contains(ratatui::style::Modifier::ITALIC),
        underline: text
            .style
            .inner
            .add_modifier
            .contains(ratatui::style::Modifier::UNDERLINED),
    }
}

fn extract_fg_bg(style: &Style) -> (Option<String>, Option<String>) {
    fn to_color_string(c: ratatui::style::Color) -> Option<String> {
        // Prefer the named-color round-trip via the matching
        // parser in `parse_color` (which accepts both named and
        // "#rrggbb" hex). For `Color::Rgb(r, g, b)` emit a hex
        // string so the renderer can reconstruct the exact color.
        use ratatui::style::Color as C;
        match c {
            C::Reset => None,
            C::Rgb(r, g, b) => Some(format!("#{:02x}{:02x}{:02x}", r, g, b)),
            _ => Some(format!("{:?}", c).to_lowercase()),
        }
    }
    let fg = style.inner.fg.and_then(to_color_string);
    let bg = style.inner.bg.and_then(to_color_string);
    (fg, bg)
}

/// Register `pairee.preview_widget(opts, widget)` on the central
/// `pairee` table. The widget argument is one of `Span`, `Line`, or
/// `Text` (or the corresponding plain-table forms). The `opts`
/// argument is a Lua table (currently unused — M4-T2 will add
/// `path`, `area`, `scroll`, `bg`).
pub fn bind(
    lua: &mlua::Lua,
    parent: &mlua::Table<'_>,
    tx: super::SendFn,
) -> mlua::Result<()> {
    let preview_fn = lua.create_function(
        move |_lua_ctx, (opts, widget): (mlua::Table, mlua::Value)| {
            // For M4-T1 the opts.path is used if provided; if
            // not, we send to the "current preview" by leaving
            // the path empty.
            let path: Option<PathBuf> = opts
                .get::<_, mlua::String>("path")
                .ok()
                .and_then(|s| s.to_str().ok().map(|c| PathBuf::from(c.to_string())));
            let plugin_widget = widget_to_plugin(widget)?;
            // The caller passes a `SendFn` (Arc<dyn Fn>) closure
            // that knows how to send the request; this decouples
            // us from the mpsc sender shape.
            tx(PluginRequest::UpdatePluginWidget {
                path: path.unwrap_or_default(),
                widget: plugin_widget,
            })
            .map_err(|e| mlua::Error::RuntimeError(format!("preview_widget: {e}")))?;
            Ok(true)
        },
    )?;
    parent.set("preview_widget", preview_fn)?;
    Ok(())
}

/// Convert a Lua value (a widget userdata) to a `PluginWidget`.
pub fn widget_to_plugin(val: mlua::Value) -> mlua::Result<PluginWidget> {
    use crate::app::state::types::PluginWidget as PW;
    match val {
        mlua::Value::UserData(ud) => {
            if let Ok(s) = ud.borrow::<Span>()        { return Ok(span_to_plugin(&s)); }
            if let Ok(l) = ud.borrow::<Line>()        { return Ok(line_to_plugin(&l)); }
            if let Ok(t) = ud.borrow::<Text>()        { return Ok(text_to_plugin(&t)); }
            if let Ok(p) = ud.borrow::<super::elements::Paragraph>() {
                return Ok(PW::Paragraph(p.text.lines.iter().map(|l|
                    l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>().join(" ")
                ).collect::<Vec<_>>().join("\n")));
            }
            if let Ok(l) = ud.borrow::<super::elements::List>() {
                return Ok(PW::List(l.items.clone()));
            }
            if let Ok(g) = ud.borrow::<super::elements::Gauge>() {
                return Ok(PW::Gauge { ratio: g.ratio, label: g.label.clone() });
            }
            if let Ok(t) = ud.borrow::<super::elements::Table>() {
                let headers: Vec<String> = t.header.as_ref()
                    .map(|r| r.cells.iter().map(|c| c.content.text.clone()).collect())
                    .unwrap_or_default();
                let rows: Vec<Vec<String>> = t.rows.iter()
                    .map(|r| r.cells.iter().map(|c| c.content.text.clone()).collect())
                    .collect();
                return Ok(PW::Table { headers, rows });
            }
            Err(mlua::Error::RuntimeError(
                "preview_widget: widget is not a recognised widget type".to_string(),
            ))
        }
        other => Err(mlua::Error::RuntimeError(format!(
            "preview_widget: expected widget userdata, got {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;
    use std::sync::Arc;

    #[test]
    fn test_widget_to_plugin_span_bold_red() {
        let lua = Lua::new();
        let ui_table = lua.create_table().unwrap();
        // Wire up the userdata-backed widget surface (Span +
        // Style) into a fresh `ui` table.
        super::super::style::bind(&lua, &ui_table).unwrap();
        super::super::elements::span::bind(&lua, &ui_table).unwrap();
        lua.globals().set("ui", ui_table).unwrap();
        let v: mlua::Value = lua
            .load("return ui.Span('hello'):fg('red'):bold()")
            .eval()
            .expect("span builds");
        let ud = v.as_userdata().expect("userdata").clone();
        let span = ud.borrow::<Span>().expect("Span borrow").clone();
        let pw = span_to_plugin(&span);
        match pw {
            PluginWidget::RichSpan {
                text,
                fg,
                bold,
                ..
            } => {
                assert_eq!(text, "hello");
                assert!(fg.is_some(), "fg should be set, got None");
                assert!(bold, "bold should be set");
            }
            other => panic!("expected RichSpan, got {other:?}"),
        }
    }

    #[test]
    fn test_widget_to_plugin_line() {
        let lua = Lua::new();
        let ui_table = lua.create_table().unwrap();
        super::super::style::bind(&lua, &ui_table).unwrap();
        super::super::elements::span::bind(&lua, &ui_table).unwrap();
        super::super::elements::line::bind(&lua, &ui_table).unwrap();
        lua.globals().set("ui", ui_table).unwrap();
        let v: mlua::Value = lua
            .load("return ui.Line('hello'):fg('red'):bold()")
            .eval()
            .expect("line builds");
        let ud = v.as_userdata().expect("userdata").clone();
        let line = ud.borrow::<Line>().expect("Line borrow").clone();
        let pw = line_to_plugin(&line);
        match pw {
            PluginWidget::RichLine { spans, .. } => {
                assert_eq!(spans.len(), 1);
            }
            other => panic!("expected RichLine, got {other:?}"),
        }
    }

    #[test]
    fn test_peek_line_with_red_bold_propagates_to_spans() {
        // The M5 done-criterion: a peek() that returns
        // `ui.Line("hi"):fg("red"):bold()` must produce a
        // PluginWidget::RichLine whose single RichSpan carries the
        // line-level fg/bold into the span.
        let lua = Lua::new();
        let ui_table = lua.create_table().unwrap();
        super::super::style::bind(&lua, &ui_table).unwrap();
        super::super::elements::line::bind(&lua, &ui_table).unwrap();
        lua.globals().set("ui", ui_table).unwrap();
        let v: mlua::Value = lua
            .load("return ui.Line('hi'):fg('red'):bold()")
            .eval()
            .expect("line builds");
        let ud = v.as_userdata().expect("userdata").clone();
        let line = ud.borrow::<Line>().expect("Line borrow").clone();
        let pw = line_to_plugin(&line);
        // The PluginWidget::RichLine should have one span (the text
        // "hi") and the inherited red fg + bold modifier.
        match pw {
            PluginWidget::RichLine {
                spans, fg, bold, ..
            } => {
                assert_eq!(spans.len(), 1);
                assert_eq!(fg.as_deref(), Some("red"));
                assert!(bold);
                // The first span should carry the same color
                // inherited from the line.
                match &spans[0] {
                    PluginWidget::RichSpan {
                        fg: span_fg,
                        text,
                        bold: span_bold,
                        ..
                    } => {
                        assert_eq!(span_fg.as_deref(), Some("red"));
                        assert_eq!(text, "hi");
                        assert!(span_bold);
                    }
                    other => panic!("expected RichSpan, got {other:?}"),
                }
            }
            other => panic!("expected RichLine, got {other:?}"),
        }
    }
}
