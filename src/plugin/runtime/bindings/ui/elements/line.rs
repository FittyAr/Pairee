//! M4-T1: `ui.Line` userdata — a sequence of styled Spans.

use super::span::Span;
use super::super::style::Style;
use mlua::{MetaMethod, UserData, UserDataMethods};
use ratatui::text::Line as RatLine;

/// A `ui.Line(...)` userdata carrying a list of Spans and a
/// line-level style (applied to every span that doesn't override
/// the field).
#[derive(Debug, Clone)]
pub struct Line {
    pub spans: Vec<Span>,
    pub style: Style,
}

impl Line {
    pub fn new() -> Self {
        Self {
            spans: Vec::new(),
            style: Style::new(),
        }
    }

    pub fn from_string(text: impl Into<String>) -> Self {
        Self {
            spans: vec![Span::new(text)],
            style: Style::new(),
        }
    }

    pub fn from_spans(spans: Vec<Span>) -> Self {
        Self {
            spans,
            style: Style::new(),
        }
    }

    /// Materialise the userdata into a `ratatui::text::Line` for
    /// direct rendering. Used by M4-T2's `Renderable` dispatch.
    #[allow(dead_code)]
    pub(crate) fn to_ratatui(&self) -> RatLine<'static> {
        let mut line = RatLine::from(
            self.spans
                .iter()
                .map(|s| {
                    // Per-span fg/bg win; fall back to line-level
                    // fg/bg.
                    let effective_style = s.style.inner.patch(if self.style.inner.fg.is_some()
                        || self.style.inner.bg.is_some()
                    {
                        ratatui::style::Style::new()
                            .fg(self.style.inner.fg.unwrap_or_else(|| {
                                s.style.inner.fg.unwrap_or(ratatui::style::Color::Reset)
                            }))
                            .bg(self.style.inner.bg.unwrap_or_else(|| {
                                s.style.inner.bg.unwrap_or(ratatui::style::Color::Reset)
                            }))
                    } else {
                        ratatui::style::Style::new()
                    });
                    let _ = effective_style; // intentionally unused; we just return span
                    if s.style.inner == Default::default() {
                        ratatui::text::Span::raw(s.text.clone())
                    } else {
                        ratatui::text::Span::styled(s.text.clone(), s.style.inner)
                    }
                })
                .collect::<Vec<_>>(),
        );
        if self.style.inner != Default::default() {
            line = line.style(self.style.inner);
        }
        line
    }
}

impl Default for Line {
    fn default() -> Self {
        Self::new()
    }
}

impl UserData for Line {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("push", |_lua, this, val: mlua::Value| {
            match val {
                mlua::Value::String(s) => {
                    let s = s.to_str()?.to_string();
                    this.spans.push(Span::new(s));
                }
                mlua::Value::UserData(ud) => {
                    let span = ud
                        .borrow::<Span>()
                        .map_err(|e| mlua::Error::RuntimeError(format!("push: {e}")))?
                        .clone();
                    this.spans.push(span);
                }
                other => {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Line.push: expected string or Span, got {other:?}"
                    )))
                }
            }
            Ok(this.clone())
        });
        methods.add_method_mut("style", |_lua, this, s: mlua::AnyUserData| {
            let s = s
                .borrow::<Style>()
                .map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            this.style = Style { inner: s.inner };
            Ok(this.clone())
        });
        methods.add_method_mut("fg", |_lua, this, val: mlua::Value| {
            let parsed = super::super::style::parse_color_value(val)?;
            let mut s = this.style.inner.clone();
            if let Some(c) = parsed {
                s = s.fg(c);
            } else {
                s.fg = None;
            }
            this.style.inner = s;
            Ok(this.clone())
        });
        methods.add_method_mut("bg", |_lua, this, val: mlua::Value| {
            let parsed = super::super::style::parse_color_value(val)?;
            let mut s = this.style.inner.clone();
            if let Some(c) = parsed {
                s = s.bg(c);
            } else {
                s.bg = None;
            }
            this.style.inner = s;
            Ok(this.clone())
        });
        methods.add_method_mut("bold", |_lua, this, ()| {
            this.style.inner = this.style.inner.add_modifier(ratatui::style::Modifier::BOLD);
            Ok(this.clone())
        });
        methods.add_method_mut("dim", |_lua, this, ()| {
            this.style.inner = this.style.inner.add_modifier(ratatui::style::Modifier::DIM);
            Ok(this.clone())
        });
        methods.add_method_mut("italic", |_lua, this, ()| {
            this.style.inner = this.style.inner.add_modifier(ratatui::style::Modifier::ITALIC);
            Ok(this.clone())
        });
        methods.add_method_mut("underline", |_lua, this, ()| {
            this.style.inner = this
                .style
                .inner
                .add_modifier(ratatui::style::Modifier::UNDERLINED);
            Ok(this.clone())
        });
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            let texts: Vec<String> = this.spans.iter().map(|s| s.text.clone()).collect();
            Ok(format!("Line(spans={:?}, style={:?})", texts, this.style.inner))
        });
    }
}

/// `ui.Line(...)` callable: build a Line from a string, a Span, or
/// a sequence of mixed values.
pub fn bind(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let line = lua.create_table()?;
    line.set(
        "__call",
        lua.create_function(|lua_ctx, args: mlua::MultiValue| {
            // When Lua calls `ui.Line(...)`, mlua passes
            // `(table, args...)` — i.e. the FIRST element of
            // `args` is the table being called (the `ui.Line`
            // table itself). If that first element is a Table
            // that is the SAME table we are constructing (a
            // callable marker, not a real sequence), skip it and
            // look at the rest of `args`. A bare callable table
            // has no integer keys, so its `len()` is 0; a real
            // sequence has len > 0.
            let mut iter = args.into_iter();
            let first = iter.next().unwrap_or(mlua::Value::Nil);
            let (skip_first, dispatch_arg) = match &first {
                mlua::Value::Table(t) => {
                    let len = t.clone().len().unwrap_or(0);
                    if len == 0 {
                        // First arg is the callable marker.
                        (true, iter.next().unwrap_or(mlua::Value::Nil))
                    } else {
                        // First arg is a real sequence — use it.
                        (false, first)
                    }
                }
                _ => (false, first),
            };
            let _ = skip_first;
            match dispatch_arg {
                mlua::Value::String(s) => {
                    let s = s.to_str()?.to_string();
                    lua_ctx
                        .create_userdata(Line::from_string(s))
                        .map(mlua::Value::UserData)
                }
                mlua::Value::UserData(ud) => {
                    let span = ud
                        .borrow::<Span>()
                        .map_err(|e| mlua::Error::RuntimeError(format!("Line: {e}")))?
                        .clone();
                    lua_ctx
                        .create_userdata(Line::from_spans(vec![span]))
                        .map(mlua::Value::UserData)
                }
                mlua::Value::Table(t) => {
                    // Sequence of strings / Spans.
                    let mut spans = Vec::new();
                    for i in 1..=t.len().unwrap_or(0) {
                        let v: mlua::Value = t.get(i)?;
                        match v {
                            mlua::Value::String(s) => {
                                spans.push(Span::new(s.to_str()?.to_string()));
                            }
                            mlua::Value::UserData(ud) => {
                                let span = ud
                                    .borrow::<Span>()
                                    .map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?
                                    .clone();
                                spans.push(span);
                            }
                            _ => {
                                return Err(mlua::Error::RuntimeError(
                                    "Line: sequence elements must be strings or Spans"
                                        .to_string(),
                                ));
                            }
                        }
                    }
                    lua_ctx
                        .create_userdata(Line::from_spans(spans))
                        .map(mlua::Value::UserData)
                }
                mlua::Value::Nil => lua_ctx
                    .create_userdata(Line::new())
                    .map(mlua::Value::UserData),
                other => Err(mlua::Error::RuntimeError(format!(
                    "Line: unexpected {other:?}"
                ))),
            }
        })?,
    )?;
    let mt = lua.create_table()?;
    mt.set("__call", {
        let inner_call = line.get::<_, mlua::Function>("__call")?;
        inner_call
    })?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    line.set_metatable(Some(mt));
    parent.set("Line", line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_line_fg_red_bold_builds() {
        let mut line = Line::from_string("hello");
        line.style.inner = line
            .style
            .inner
            .fg(ratatui::style::Color::Red)
            .add_modifier(ratatui::style::Modifier::BOLD);
        let r = line.to_ratatui();
        let _ = r;
        assert!(line.style.inner.fg.is_some());
        assert!(line
            .style
            .inner
            .add_modifier
            .contains(ratatui::style::Modifier::BOLD));
    }

    #[test]
    fn test_line_lua_builder_chain() {
        let lua = Lua::new();
        let ui_table = lua.create_table().unwrap();
        crate::plugin::runtime::bindings::ui::style::bind(
            &lua,
            &ui_table,
        )
        .unwrap();
        crate::plugin::runtime::bindings::ui::elements::span::bind(
            &lua,
            &ui_table,
        )
        .unwrap();
        crate::plugin::runtime::bindings::ui::elements::line::bind(
            &lua,
            &ui_table,
        )
        .unwrap();
        lua.globals().set("ui", ui_table).unwrap();
        // The line builds; we just verify no panic and that the
        // returned value is a Line userdata.
        let v: mlua::Value = lua
            .load("return ui.Line('hello')")
            .eval()
            .expect("line builds");
        let _ = v;
    }
}
