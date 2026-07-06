//! M4-T1: `ui.Span` userdata — a styled text run. Builder pattern
//! returns `self` (cloned) on every mutator.

use super::super::style::Style;
use mlua::{MetaMethod, UserData, UserDataMethods};
use ratatui::text::Span as RatSpan;

/// A `ui.Span(text)` userdata carrying the text and an optional
/// style.
#[derive(Debug, Clone)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

impl Span {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::new(),
        }
    }

    /// Materialise the userdata into a `ratatui::text::Span` for
    /// direct rendering. Used by M4-T2's `Renderable` dispatch.
    #[allow(dead_code)]
    pub(crate) fn to_ratatui(&self) -> RatSpan<'static> {
        if self.style.inner == Default::default() {
            RatSpan::raw(self.text.clone())
        } else {
            RatSpan::styled(self.text.clone(), self.style.inner)
        }
    }
}

impl UserData for Span {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("text", |_lua, this, ()| Ok(this.text.clone()));
        methods.add_method("style", |_lua, this, ()| Ok(this.style.clone()));
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
        // Modifier folds
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
        methods.add_method_mut("blink", |_lua, this, ()| {
            this.style.inner = this.style.inner.add_modifier(ratatui::style::Modifier::SLOW_BLINK);
            Ok(this.clone())
        });
        methods.add_method_mut("reverse", |_lua, this, ()| {
            this.style.inner = this.style.inner.add_modifier(ratatui::style::Modifier::REVERSED);
            Ok(this.clone())
        });
        methods.add_method_mut("hidden", |_lua, this, ()| {
            this.style.inner = this.style.inner.add_modifier(ratatui::style::Modifier::HIDDEN);
            Ok(this.clone())
        });
        methods.add_method_mut("crossed", |_lua, this, ()| {
            this.style.inner = this
                .style
                .inner
                .add_modifier(ratatui::style::Modifier::CROSSED_OUT);
            Ok(this.clone())
        });
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(format!(
                "Span(text={:?}, style={:?})",
                this.text, this.style.inner
            ))
        });
    }
}

/// `ui.Span(text)` callable: build a new Span from a string.
pub fn bind(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let span = lua.create_table()?;
    span.set(
        "__call",
        lua.create_function(|lua, args: mlua::MultiValue| {
            // Lua passes (table, args...) to __call. The first
            // element is the callable marker; skip it and look
            // at the rest. A bare callable table has no integer
            // keys (len == 0).
            let mut iter = args.into_iter();
            let first = iter.next().unwrap_or(mlua::Value::Nil);
            let dispatch_arg = match &first {
                mlua::Value::Table(t) => {
                    let len = t.clone().len().unwrap_or(0);
                    if len == 0 {
                        iter.next().unwrap_or(mlua::Value::Nil)
                    } else {
                        first
                    }
                }
                _ => first,
            };
            let text: String = match dispatch_arg {
                mlua::Value::String(s) => s.to_str().ok().map(|c| c.to_string()).unwrap_or_default(),
                _ => String::new(),
            };
            lua.create_userdata(Span::new(text)).map(mlua::Value::UserData)
        })?,
    )?;
    let mt = lua.create_table()?;
    let inner_call = span.get::<_, mlua::Function>("__call")?;
    mt.set("__call", inner_call)?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    span.set_metatable(Some(mt));
    parent.set("Span", span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_to_ratatui_raw() {
        let span = Span::new("hi");
        let r = span.to_ratatui();
        assert_eq!(r.content, "hi");
    }
}
