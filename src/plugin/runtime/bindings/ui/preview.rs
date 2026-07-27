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
                span_to_plugin_with_inheritance(s, line_fg.clone(), line_bg.clone(), line_modifiers)
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
        bold: text
            .style
            .inner
            .add_modifier
            .contains(ratatui::style::Modifier::BOLD),
        dim: text
            .style
            .inner
            .add_modifier
            .contains(ratatui::style::Modifier::DIM),
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

/// §N1/N2: hard caps on a `PluginWidget` before it crosses the
/// plugin-to-main-loop boundary. Without these a malicious plugin
/// can build a deeply nested widget tree (stack-overflow on
/// recursive decoders) or a single widget holding a multi-GB
/// string (OOM in the renderer). The caps are intentionally
/// generous for real-world previews (8 levels of nesting, 256 KB
/// per string) but bounded.
pub const MAX_WIDGET_DEPTH: usize = 8;
pub const MAX_WIDGET_STRING_BYTES: usize = 256 * 1024;

/// Walk a `PluginWidget` in place, truncating any string that
/// exceeds `MAX_WIDGET_STRING_BYTES` and replacing any
/// over-nested `Line` / `RichLine` / `RichText` with a flat
/// placeholder line. The recursion is bounded by
/// `MAX_WIDGET_DEPTH` (8), which is well within the stack budget
/// (≈ 8 * sizeof(PluginWidget) ≈ 4 KB worst case).
pub fn sanitize_plugin_widget(widget: &mut PluginWidget) {
    sanitize_widget_at_depth(widget, 0);
}

fn sanitize_widget_at_depth(widget: &mut PluginWidget, depth: usize) {
    if depth >= MAX_WIDGET_DEPTH {
        *widget = PluginWidget::Span {
            text: "[widget tree too deep]".to_string(),
            style: String::new(),
        };
        return;
    }
    match widget {
        PluginWidget::Paragraph(s) | PluginWidget::Span { text: s, .. } => {
            truncate_string_in_place(s);
        }
        PluginWidget::RichSpan { text, .. } => {
            truncate_string_in_place(text);
        }
        PluginWidget::List(items) => {
            for item in items.iter_mut() {
                truncate_string_in_place(item);
            }
        }
        PluginWidget::Table { headers, rows } => {
            for h in headers.iter_mut() {
                truncate_string_in_place(h);
            }
            for row in rows.iter_mut() {
                for cell in row.iter_mut() {
                    truncate_string_in_place(cell);
                }
            }
        }
        PluginWidget::Gauge { label, .. } => {
            truncate_string_in_place(label);
        }
        PluginWidget::Line(spans) | PluginWidget::RichLine { spans, .. } => {
            for child in spans.iter_mut() {
                sanitize_widget_at_depth(child, depth + 1);
            }
        }
        PluginWidget::RichText { lines, .. } => {
            for child in lines.iter_mut() {
                sanitize_widget_at_depth(child, depth + 1);
            }
        }
    }
}

fn truncate_string_in_place(s: &mut String) {
    if s.len() > MAX_WIDGET_STRING_BYTES {
        // Truncate at a char boundary so we never split a
        // multi-byte codepoint.
        let mut idx = MAX_WIDGET_STRING_BYTES;
        while !s.is_char_boundary(idx) {
            idx -= 1;
        }
        s.truncate(idx);
        s.push_str("…");
    }
}

/// Register `pairee.preview_widget(opts, widget)` on the central
/// `pairee` table. The widget argument is one of `Span`, `Line`, or
/// `Text` (or the corresponding plain-table forms). The `opts`
/// argument is a Lua table (currently unused — M4-T2 will add
/// `path`, `area`, `scroll`, `bg`).
pub fn bind(lua: &mlua::Lua, parent: &mlua::Table<'_>, tx: super::SendFn) -> mlua::Result<()> {
    let preview_fn = lua.create_function(
        move |_lua_ctx, (opts, widget): (mlua::Table, mlua::Value)| {
            // For M4-T1 the opts.path is used if provided; if
            // not, we send to the "current preview" by leaving
            // the path empty.
            let path: Option<PathBuf> = opts
                .get::<_, mlua::String>("path")
                .ok()
                .and_then(|s| s.to_str().ok().map(|c| PathBuf::from(c.to_string())));
            let mut plugin_widget = widget_to_plugin(widget)?;
            // §N1/N2: cap depth + truncate oversized strings
            // before the widget crosses the plugin-to-main-loop
            // boundary (DoS guard).
            sanitize_plugin_widget(&mut plugin_widget);
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
            if let Ok(s) = ud.borrow::<Span>() {
                return Ok(span_to_plugin(&s));
            }
            if let Ok(l) = ud.borrow::<Line>() {
                return Ok(line_to_plugin(&l));
            }
            if let Ok(t) = ud.borrow::<Text>() {
                return Ok(text_to_plugin(&t));
            }
            if let Ok(p) = ud.borrow::<super::elements::paragraph::Paragraph>() {
                // Convert the Paragraph's underlying Text to a
                // RichText so each line preserves its per-span styles.
                // The paragraph-level style (alignment, wrap) is
                // not preserved in the legacy PluginWidget::Paragraph
                // path; the M4-T2 Renderable dispatch renders the
                // raw RichText directly without loss.
                return Ok(text_to_plugin(&p.text));
            }
            if let Ok(l) = ud.borrow::<super::elements::list::List>() {
                return Ok(PW::List(l.items.clone()));
            }
            if let Ok(g) = ud.borrow::<super::elements::gauge::Gauge>() {
                return Ok(PW::Gauge {
                    ratio: g.ratio,
                    label: g.label.clone(),
                });
            }
            if let Ok(t) = ud.borrow::<super::elements::table::Table>() {
                let headers: Vec<String> = t
                    .header
                    .as_ref()
                    .map(|r| r.cells.iter().map(|c| c.content.text.clone()).collect())
                    .unwrap_or_default();
                let rows: Vec<Vec<String>> = t
                    .rows
                    .iter()
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
            PluginWidget::RichSpan { text, fg, bold, .. } => {
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

    // §N1: deeply nested widget trees must be flattened at the
    // `MAX_WIDGET_DEPTH` boundary so a malicious plugin cannot
    // blow the stack on the recursive renderer / decoder.
    #[test]
    fn test_sanitize_plugin_widget_truncates_deep_nesting() {
        // Build a Line nesting MAX_WIDGET_DEPTH + 4 levels.
        let mut deepest = PluginWidget::Span {
            text: "leaf".to_string(),
            style: String::new(),
        };
        for _ in 0..(MAX_WIDGET_DEPTH + 4) {
            deepest = PluginWidget::Line(vec![deepest]);
        }
        sanitize_plugin_widget(&mut deepest);
        // After sanitising, the tree must not exceed the cap.
        // `MAX_WIDGET_DEPTH` is the max number of nested
        // CONTAINERS (Line / RichLine / RichText); the leaf
        // (Span / RichSpan / Paragraph / etc.) always adds one
        // more level.
        fn depth(w: &PluginWidget) -> usize {
            match w {
                PluginWidget::Line(s) | PluginWidget::RichLine { spans: s, .. } => {
                    1 + s.iter().map(depth).max().unwrap_or(0)
                }
                PluginWidget::RichText { lines, .. } => {
                    1 + lines.iter().map(depth).max().unwrap_or(0)
                }
                _ => 1,
            }
        }
        assert!(
            depth(&deepest) <= MAX_WIDGET_DEPTH + 1,
            "sanitised tree depth {} exceeded cap {}+1",
            depth(&deepest),
            MAX_WIDGET_DEPTH
        );
    }

    // §N2: oversize strings in a widget are truncated to
    // `MAX_WIDGET_STRING_BYTES` plus a trailing ellipsis. A
    // malicious plugin sending a multi-GB string to a Paragraph
    // must not be able to OOM the renderer.
    #[test]
    fn test_sanitize_plugin_widget_truncates_oversize_strings() {
        let huge = "x".repeat(MAX_WIDGET_STRING_BYTES * 4);
        let mut widget = PluginWidget::Paragraph(huge);
        sanitize_plugin_widget(&mut widget);
        match widget {
            PluginWidget::Paragraph(s) => {
                // The truncated length is at most the cap plus the
                // single-character ellipsis suffix.
                assert!(
                    s.len() <= MAX_WIDGET_STRING_BYTES + "…".len(),
                    "paragraph not truncated, got len={}",
                    s.len()
                );
                assert!(s.ends_with('…'), "truncation must append the ellipsis");
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    // §N1 follow-up: cap also applies to Line/Table/List children.
    #[test]
    fn test_sanitize_plugin_widget_truncates_table_cells() {
        let huge = "a".repeat(MAX_WIDGET_STRING_BYTES * 2);
        let mut widget = PluginWidget::Table {
            headers: vec![huge.clone()],
            rows: vec![vec![huge]],
        };
        sanitize_plugin_widget(&mut widget);
        match widget {
            PluginWidget::Table { headers, rows } => {
                for h in headers {
                    assert!(h.len() <= MAX_WIDGET_STRING_BYTES + "…".len());
                }
                for row in rows {
                    for cell in row {
                        assert!(cell.len() <= MAX_WIDGET_STRING_BYTES + "…".len());
                    }
                }
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }
}
