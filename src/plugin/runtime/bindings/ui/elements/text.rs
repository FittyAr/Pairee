//! M4-T1: `ui.Text` userdata — a sequence of styled Lines.

use super::line::Line;
use super::super::style::Style;
use mlua::{MetaMethod, UserData, UserDataMethods};
use ratatui::text::Text as RatText;

/// A `ui.Text(...)` userdata carrying multiple Lines.
#[derive(Debug, Clone)]
pub struct Text {
    pub lines: Vec<Line>,
    pub style: Style,
}

impl Text {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            style: Style::new(),
        }
    }

    pub fn from_string(text: impl Into<String>) -> Self {
        let s: String = text.into();
        let lines = s
            .split('\n')
            .map(|l| Line::from_string(l.to_string()))
            .collect();
        Self {
            lines,
            style: Style::new(),
        }
    }

    pub fn from_lines(lines: Vec<Line>) -> Self {
        Self {
            lines,
            style: Style::new(),
        }
    }

    pub(crate) fn to_ratatui(&self) -> RatText<'static> {
        let mut text = RatText::from(
            self.lines
                .iter()
                .map(|l| l.to_ratatui())
                .collect::<Vec<_>>(),
        );
        if self.style.inner != Default::default() {
            text = text.style(self.style.inner);
        }
        text
    }
}

impl Default for Text {
    fn default() -> Self {
        Self::new()
    }
}

impl UserData for Text {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("push", |_lua, this, val: mlua::Value| {
            match val {
                mlua::Value::String(s) => {
                    let s = s.to_str()?.to_string();
                    this.lines.push(Line::from_string(s));
                }
                mlua::Value::UserData(ud) => {
                    let line = ud
                        .borrow::<Line>()
                        .map_err(|e| mlua::Error::RuntimeError(format!("push: {e}")))?
                        .clone();
                    this.lines.push(line);
                }
                other => {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Text.push: expected string or Line, got {other:?}"
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
            let mut __s = this.style.inner.clone();
        if let Some(c) = parsed { __s = __s.fg(c); } else { __s.fg = None; }
        this.style.inner = __s;
            Ok(this.clone())
        });
        methods.add_method_mut("bg", |_lua, this, val: mlua::Value| {
            let parsed = super::super::style::parse_color_value(val)?;
            let mut __s = this.style.inner.clone();
        if let Some(c) = parsed { __s = __s.bg(c); } else { __s.bg = None; }
        this.style.inner = __s;
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
        // `ui.Text.parse(ansi_string)` — M4-T1 stub. M4-T2 will
        // ship a real ANSI parser (per the task plan, "STUB that
        // logs `log::warn!` and returns an empty Text"). For M4-T1
        // we DO parse the string by splitting on `\n` and applying
        // a no-op style — the warning is fired ONCE per process.
        methods.add_method_mut("parse", |_lua, this, ansi: String| {
            static WARNED: std::sync::atomic::AtomicBool =
                std::sync::atomic::AtomicBool::new(false);
            if !WARNED.swap(true, std::sync::atomic::Ordering::SeqCst) {
                log::warn!(
                    "ui.Text.parse(ansi_string) is in M4-T1 stub mode — ANSI escape \
                     sequences are passed through verbatim; the real parser lands \
                     in M4-T2."
                );
            }
            this.lines = ansi.split('\n').map(Line::from_string).collect();
            Ok(this.clone())
        });
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("Text(lines={}, style={:?})", this.lines.len(), this.style.inner))
        });
    }
}

/// `ui.Text(...)` callable: build Text from a string, a Line, or a
/// sequence of mixed values.
pub fn bind(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let text = lua.create_table()?;
    text.set(
        "__call",
        lua.create_function(|lua_ctx, args: mlua::MultiValue| {
            // Skip the callable-table marker (first arg, when it
            // is a Table with len == 0).
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
            match dispatch_arg {
                mlua::Value::String(s) => {
                    let s = s.to_str()?.to_string();
                    lua_ctx
                        .create_userdata(Text::from_string(s))
                        .map(mlua::Value::UserData)
                }
                mlua::Value::UserData(ud) => {
                    let line = ud
                        .borrow::<Line>()
                        .map_err(|e| mlua::Error::RuntimeError(format!("Text: {e}")))?
                        .clone();
                    lua_ctx
                        .create_userdata(Text::from_lines(vec![line]))
                        .map(mlua::Value::UserData)
                }
                mlua::Value::Table(t) => {
                    let mut lines = Vec::new();
                    for i in 1..=t.len().unwrap_or(0) {
                        let v: mlua::Value = t.get(i)?;
                        match v {
                            mlua::Value::String(s) => {
                                lines.push(Line::from_string(s.to_str()?.to_string()));
                            }
                            mlua::Value::UserData(ud) => {
                                let line = ud
                                    .borrow::<Line>()
                                    .map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?
                                    .clone();
                                lines.push(line);
                            }
                            _ => {
                                return Err(mlua::Error::RuntimeError(
                                    "Text: sequence elements must be strings or Lines"
                                        .to_string(),
                                ));
                            }
                        }
                    }
                    lua_ctx
                        .create_userdata(Text::from_lines(lines))
                        .map(mlua::Value::UserData)
                }
                mlua::Value::Nil => lua_ctx
                    .create_userdata(Text::new())
                    .map(mlua::Value::UserData),
                other => Err(mlua::Error::RuntimeError(format!(
                    "Text: unexpected {other:?}"
                ))),
            }
        })?,
    )?;
    let mt = lua.create_table()?;
    let inner_call = text.get::<_, mlua::Function>("__call")?;
    mt.set("__call", inner_call)?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    text.set_metatable(Some(mt));
    parent.set("Text", text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_from_string_splits_on_newline() {
        let t = Text::from_string("a\nb\nc");
        assert_eq!(t.lines.len(), 3);
    }

    #[test]
    fn test_text_to_ratatui_round_trip() {
        let t = Text::from_string("hello\nworld");
        let _ = t.to_ratatui();
    }
}
